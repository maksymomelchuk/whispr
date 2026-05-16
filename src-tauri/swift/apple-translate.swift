import Foundation

// ── JSON I/O ─────────────────────────────────────────────────────────────────

struct TranslateRequest: Codable {
    let text: String
    let source: String?  // nil → framework auto-detects source language
    let target: String
}

struct TranslateResponse: Codable {
    let translated: String?
    let error_code: String?
    let error: String?
}

func respond(_ r: TranslateResponse) -> Never {
    let encoder = JSONEncoder()
    guard let data = try? encoder.encode(r),
          let str = String(data: data, encoding: .utf8)
    else {
        print("{\"translated\":null,\"error_code\":\"encode_error\",\"error\":\"Failed to encode response\"}")
        exit(2)
    }
    print(str)
    exit(r.translated != nil ? 0 : 1)
}

func logStep(_ message: String) {
    FileHandle.standardError.write(Data("[apple-translate] \(message)\n".utf8))
}

// ── Read request from stdin ──────────────────────────────────────────────────

let inputData = FileHandle.standardInput.readDataToEndOfFile()
guard let request = try? JSONDecoder().decode(TranslateRequest.self, from: inputData) else {
    respond(TranslateResponse(translated: nil, error_code: "invalid_input", error: "Invalid JSON input"))
}

// ── Translation ──────────────────────────────────────────────────────────────

#if canImport(Translation)
import Translation

if #available(macOS 26.0, *) {
    // The programmatic TranslationSession initializer requires a known source
    // language. Auto-detect is only available via SwiftUI's .translationTask,
    // which we can't use from a command-line sidecar.
    guard let sourceCode = request.source else {
        respond(TranslateResponse(
            translated: nil,
            error_code: "source_required",
            error: "Apple Translate requires an explicit source language. Set the mode's Spoken Language to a specific language (not Auto)."
        ))
    }

    let sourceLang = Locale.Language(languageCode: Locale.LanguageCode(sourceCode))
    let targetLang = Locale.Language(languageCode: Locale.LanguageCode(request.target))

    // CLI sidecar pattern: pump the main runloop while the Swift Concurrency
    // task runs. TranslationSession's translate() delivers its result via the
    // main runloop (likely XPC-backed), so blocking on a DispatchSemaphore
    // without pumping deadlocks. Schedule a hard 30 s timeout via DispatchQueue.
    let runLoop = CFRunLoopGetCurrent()
    let timeoutWork = DispatchWorkItem {
        respond(TranslateResponse(
            translated: nil,
            error_code: "translation_failed",
            error: "Translation timed out."
        ))
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 30, execute: timeoutWork)

    logStep("starting task: \(sourceCode) → \(request.target)")

    Task {
        do {
            // We deliberately skip LanguageAvailability().status(...) here:
            // on macOS 26 from a CLI process (no SwiftUI environment) that
            // call hangs forever. session.translate's own error is surfaced
            // instead — including "language pack not installed".
            logStep("creating TranslationSession")
            let session = TranslationSession(installedSource: sourceLang, target: targetLang)
            logStep("calling session.translate")
            let result = try await session.translate(request.text)
            logStep("translation complete")
            respond(TranslateResponse(translated: result.targetText, error_code: nil, error: nil))
        } catch {
            let description = error.localizedDescription
            logStep("translate failed: \(description)")
            // Surface a structured "model_not_installed" when the framework
            // says so, so the UI can prompt the user to download the pack.
            let lower = description.lowercased()
            let code: String
            // "Unable to Translate" is the framework's generic message when
            // the language pack for the requested pair isn't installed.
            if lower.contains("not installed") || lower.contains("download")
                || lower.contains("not supported on this device")
                || lower.contains("unavailable")
                || lower.contains("unable to translate")
            {
                code = "model_not_installed"
            } else {
                code = "translation_failed"
            }
            respond(TranslateResponse(
                translated: nil,
                error_code: code,
                error: description
            ))
        }
        // Unreached in practice: respond() exits the process. Kept as a
        // safety net in case future edits make the task return normally.
        timeoutWork.cancel()
        CFRunLoopStop(runLoop)
    }

    CFRunLoopRun()
    // CFRunLoopRun returns when the timeout work item or the Task's
    // respond() has exited the process. If we somehow get here, exit.
    respond(TranslateResponse(
        translated: nil,
        error_code: "translation_failed",
        error: "Translation timed out (runloop exited unexpectedly)."
    ))
} else {
    respond(TranslateResponse(
        translated: nil,
        error_code: "requires_macos_26",
        error: "Apple Translate requires macOS 26 (Tahoe) or later."
    ))
}

#else
respond(TranslateResponse(
    translated: nil,
    error_code: "requires_macos_26",
    error: "Apple Translate requires macOS 26 (Tahoe) or later."
))
#endif
