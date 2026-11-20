use crate::recorder::AudioFormat;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
    kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey, kAudioTapPropertyFormat,
    AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
    AudioDeviceStart, AudioDeviceStop, AudioHardwareCreateAggregateDevice,
    AudioHardwareDestroyAggregateDevice, AudioObjectGetPropertyData, AudioObjectID,
    AudioObjectPropertyAddress, CATapDescription, CATapMuteBehavior,
};
use objc2_core_audio_types::{AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString, NSUUID};
use std::ffi::{c_void, CStr};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

const AGGREGATE_DEVICE_NAME: &str = "Whispr Meeting Capture";
const CREATE_PROCESS_TAP_SYMBOL: &CStr = c"AudioHardwareCreateProcessTap";
const DESTROY_PROCESS_TAP_SYMBOL: &CStr = c"AudioHardwareDestroyProcessTap";
const NO_ERROR: i32 = 0;

type CreateProcessTapFn = unsafe extern "C" fn(*const CATapDescription, *mut AudioObjectID) -> i32;
type DestroyProcessTapFn = unsafe extern "C" fn(AudioObjectID) -> i32;
type IoBlock = RcBlock<
    dyn Fn(
        NonNull<AudioTimeStamp>,
        NonNull<AudioBufferList>,
        NonNull<AudioTimeStamp>,
        NonNull<AudioBufferList>,
        NonNull<AudioTimeStamp>,
    ),
>;

/// Records the mix of everything the Mac plays (the remote side of a call)
/// through a Core Audio process tap. Dropping it tears the tap down.
pub struct SystemAudioCapture {
    tap_id: AudioObjectID,
    aggregate_id: AudioObjectID,
    io_proc_id: AudioDeviceIOProcID,
    chunk_sender: SharedSender,
    _io_block: IoBlock,
}

/// Core Audio may keep its own copy of the IO block alive after the IOProc is
/// destroyed, so the sender lives in a slot that `Drop` empties; that is what
/// closes the channel and lets the consumer finish.
type SharedSender = Arc<Mutex<Option<UnboundedSender<Vec<i16>>>>>;

// The fields are Core Audio object IDs plus the IO block, which Core Audio
// invokes on its own realtime thread regardless of who owns this struct; the
// owner only ever stops and destroys the objects, which the HAL serializes.
unsafe impl Send for SystemAudioCapture {}

impl SystemAudioCapture {
    pub fn start(chunk_tx: UnboundedSender<Vec<i16>>) -> Result<(Self, AudioFormat), String> {
        let tap_api = ProcessTapApi::resolve()?;
        let description = global_tap_description();
        let tap_id = tap_api.create(&description)?;
        let format = read_tap_format(tap_id).inspect_err(|_| tap_api.destroy(tap_id))?;
        let aggregate_id =
            create_tap_aggregate(&description).inspect_err(|_| tap_api.destroy(tap_id))?;
        let chunk_sender: SharedSender = Arc::new(Mutex::new(Some(chunk_tx)));
        let io_block = io_block(chunk_sender.clone());
        let io_proc_id = start_io(aggregate_id, &io_block).inspect_err(|_| {
            unsafe { AudioHardwareDestroyAggregateDevice(aggregate_id) };
            tap_api.destroy(tap_id);
        })?;
        let capture = Self {
            tap_id,
            aggregate_id,
            io_proc_id,
            chunk_sender,
            _io_block: io_block,
        };
        Ok((capture, format))
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        unsafe {
            AudioDeviceStop(self.aggregate_id, self.io_proc_id);
            AudioDeviceDestroyIOProcID(self.aggregate_id, self.io_proc_id);
            AudioHardwareDestroyAggregateDevice(self.aggregate_id);
        }
        self.chunk_sender.lock().unwrap().take();
        if let Ok(tap_api) = ProcessTapApi::resolve() {
            tap_api.destroy(self.tap_id);
        }
    }
}

/// Process taps exist only on macOS 14.2+, while the app deploys to 10.15, so
/// the two tap functions are looked up at runtime: a direct reference would
/// make dyld refuse to launch the whole app on older systems.
struct ProcessTapApi {
    create: CreateProcessTapFn,
    destroy: DestroyProcessTapFn,
}

impl ProcessTapApi {
    fn resolve() -> Result<Self, String> {
        let create = lookup_symbol(CREATE_PROCESS_TAP_SYMBOL);
        let destroy = lookup_symbol(DESTROY_PROCESS_TAP_SYMBOL);
        if create.is_null() || destroy.is_null() {
            return Err("System audio capture requires macOS 14.2 or later".to_string());
        }
        unsafe {
            Ok(Self {
                create: std::mem::transmute::<*mut c_void, CreateProcessTapFn>(create),
                destroy: std::mem::transmute::<*mut c_void, DestroyProcessTapFn>(destroy),
            })
        }
    }

    fn create(&self, description: &CATapDescription) -> Result<AudioObjectID, String> {
        let mut tap_id: AudioObjectID = 0;
        let status = unsafe { (self.create)(description, &mut tap_id) };
        if status != NO_ERROR {
            return Err(format!(
                "Creating the system audio tap failed (OSStatus {status}). Allow Whispr under \
                 System Settings → Privacy & Security → Screen & System Audio Recording."
            ));
        }
        Ok(tap_id)
    }

    fn destroy(&self, tap_id: AudioObjectID) {
        unsafe { (self.destroy)(tap_id) };
    }
}

fn lookup_symbol(name: &CStr) -> *mut c_void {
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
}

fn global_tap_description() -> Retained<CATapDescription> {
    let excluded_processes: Retained<NSArray<NSNumber>> = NSArray::new();
    unsafe {
        let description = CATapDescription::initMonoGlobalTapButExcludeProcesses(
            CATapDescription::alloc(),
            &excluded_processes,
        );
        description.setPrivate(true);
        description.setMuteBehavior(CATapMuteBehavior::Unmuted);
        description
    }
}

fn read_tap_format(tap_id: AudioObjectID) -> Result<AudioFormat, String> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioTapPropertyFormat,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut description: AudioStreamBasicDescription = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            tap_id,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut description).cast(),
        )
    };
    if status != NO_ERROR {
        return Err(format!(
            "Reading the system audio format failed (OSStatus {status})"
        ));
    }
    Ok(AudioFormat {
        sample_rate: description.mSampleRate as u32,
        channels: description.mChannelsPerFrame as u16,
    })
}

fn create_tap_aggregate(description: &CATapDescription) -> Result<AudioObjectID, String> {
    let tap_uid = unsafe { description.UUID() }.UUIDString();
    let tap_entry = string_keyed_dictionary(&[
        (
            kAudioSubTapUIDKey,
            Retained::into_super(Retained::into_super(tap_uid)),
        ),
        (kAudioSubTapDriftCompensationKey, number_object(true)),
    ]);
    let aggregate = string_keyed_dictionary(&[
        (
            kAudioAggregateDeviceNameKey,
            string_object(AGGREGATE_DEVICE_NAME),
        ),
        (
            kAudioAggregateDeviceUIDKey,
            string_object(&NSUUID::new().UUIDString().to_string()),
        ),
        (kAudioAggregateDeviceIsPrivateKey, number_object(true)),
        (kAudioAggregateDeviceTapAutoStartKey, number_object(true)),
        (kAudioAggregateDeviceTapListKey, array_object(tap_entry)),
    ]);
    let mut aggregate_id: AudioObjectID = 0;
    // NSDictionary is toll-free bridged to CFDictionary.
    let cf_aggregate = unsafe { &*(Retained::as_ptr(&aggregate) as *const CFDictionary) };
    let status = unsafe {
        AudioHardwareCreateAggregateDevice(cf_aggregate, NonNull::from(&mut aggregate_id))
    };
    if status != NO_ERROR {
        return Err(format!(
            "Creating the system audio device failed (OSStatus {status})"
        ));
    }
    Ok(aggregate_id)
}

fn string_keyed_dictionary(
    entries: &[(&CStr, Retained<AnyObject>)],
) -> Retained<NSDictionary<NSString, AnyObject>> {
    let keys: Vec<Retained<NSString>> = entries
        .iter()
        .map(|(key, _)| NSString::from_str(&key.to_string_lossy()))
        .collect();
    let key_refs: Vec<&NSString> = keys.iter().map(|key| &**key).collect();
    let values: Vec<&AnyObject> = entries.iter().map(|(_, value)| &**value).collect();
    NSDictionary::from_slices(&key_refs, &values)
}

fn string_object(value: &str) -> Retained<AnyObject> {
    Retained::into_super(Retained::into_super(NSString::from_str(value)))
}

fn number_object(value: bool) -> Retained<AnyObject> {
    Retained::into_super(Retained::into_super(Retained::into_super(
        NSNumber::new_bool(value),
    )))
}

fn array_object(entry: Retained<NSDictionary<NSString, AnyObject>>) -> Retained<AnyObject> {
    Retained::into_super(Retained::into_super(NSArray::from_retained_slice(&[entry])))
}

fn io_block(chunk_sender: SharedSender) -> IoBlock {
    RcBlock::new(
        move |_now: NonNull<AudioTimeStamp>,
              input: NonNull<AudioBufferList>,
              _input_time: NonNull<AudioTimeStamp>,
              _output: NonNull<AudioBufferList>,
              _output_time: NonNull<AudioTimeStamp>| {
            let chunk = unsafe { float_buffers_to_i16(input) };
            if chunk.is_empty() {
                return;
            }
            // try_lock: the realtime audio thread must never block; the slot
            // is only contended for the instant Drop empties it.
            if let Ok(slot) = chunk_sender.try_lock() {
                if let Some(sender) = slot.as_ref() {
                    let _ = sender.send(chunk);
                }
            }
        },
    )
}

/// Tap buffers are 32-bit float; with a mono tap every buffer holds the full
/// mixdown, so the first buffer is the whole signal.
unsafe fn float_buffers_to_i16(list: NonNull<AudioBufferList>) -> Vec<i16> {
    let list = list.as_ref();
    if list.mNumberBuffers == 0 {
        return Vec::new();
    }
    let buffer = list.mBuffers[0];
    if buffer.mData.is_null() {
        return Vec::new();
    }
    let sample_count = buffer.mDataByteSize as usize / std::mem::size_of::<f32>();
    let samples = std::slice::from_raw_parts(buffer.mData as *const f32, sample_count);
    samples
        .iter()
        .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn start_io(aggregate_id: AudioObjectID, block: &IoBlock) -> Result<AudioDeviceIOProcID, String> {
    let mut io_proc_id: AudioDeviceIOProcID = None;
    let block_ptr = RcBlock::as_ptr(block) as *mut _;
    let status = unsafe {
        AudioDeviceCreateIOProcIDWithBlock(
            NonNull::from(&mut io_proc_id),
            aggregate_id,
            None,
            block_ptr,
        )
    };
    if status != NO_ERROR {
        return Err(format!(
            "Registering the system audio callback failed (OSStatus {status})"
        ));
    }
    let status = unsafe { AudioDeviceStart(aggregate_id, io_proc_id) };
    if status != NO_ERROR {
        unsafe { AudioDeviceDestroyIOProcID(aggregate_id, io_proc_id) };
        return Err(format!(
            "Starting system audio capture failed (OSStatus {status})"
        ));
    }
    Ok(io_proc_id)
}
