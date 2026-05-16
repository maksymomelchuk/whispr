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

// ── Read request from stdin ──────────────────────────────────────────────────

let inputData = FileHandle.standardInput.readDataToEndOfFile()
guard let request = try? JSONDecoder().decode(TranslateRequest.self, from: inputData) else {
    respond(TranslateResponse(translated: nil, error_code: "invalid_input", error: "Invalid JSON input"))
}

// ── Translation ──────────────────────────────────────────────────────────────

#if canImport(Translation)
import Translation

if #available(macOS 15.0, *) {
    let sourceLang: Locale.Language? = request.source.map {
        Locale.Language(languageCode: Locale.LanguageCode($0))
    }
    let targetLang = Locale.Language(languageCode: Locale.LanguageCode(request.target))

    // Configuration accepts nil source for auto-detection.
    let config = TranslationSession.Configuration(source: sourceLang, target: targetLang)

    let semaphore = DispatchSemaphore(value: 0)

    Task {
        do {
            // Check language pack availability when source is known.
            // When source is nil we skip this check (auto-detect path) and let
            // the framework return a descriptive error if the pack is missing.
            if let src = sourceLang {
                let availability = LanguageAvailability()
                let status = await availability.status(from: src, to: targetLang)
                switch status {
                case .installed:
                    break
                case .supported:
                    // Pack exists in the catalog but hasn't been downloaded yet.
                    respond(TranslateResponse(
                        translated: nil,
                        error_code: "model_not_installed",
                        error: "Translation language pack not installed. Open System Settings › General › Language & Region to download it."
                    ))
                case .unsupported:
                    respond(TranslateResponse(
                        translated: nil,
                        error_code: "unsupported_pair",
                        error: "Apple Translate does not support this language pair."
                    ))
                @unknown default:
                    respond(TranslateResponse(
                        translated: nil,
                        error_code: "unsupported_pair",
                        error: "Unknown language availability status."
                    ))
                }
            }

            let session = TranslationSession(configuration: config)
            let result = try await session.translate(request.text)
            respond(TranslateResponse(translated: result.targetString, error_code: nil, error: nil))
        } catch {
            respond(TranslateResponse(
                translated: nil,
                error_code: "translation_failed",
                error: error.localizedDescription
            ))
        }
        semaphore.signal()
    }

    // Block the main thread until the async Task completes (or 30 s timeout).
    _ = semaphore.wait(timeout: .now() + 30)
    // respond() calls exit(), so reaching here means the task timed out.
    respond(TranslateResponse(
        translated: nil,
        error_code: "translation_failed",
        error: "Translation timed out."
    ))
} else {
    respond(TranslateResponse(
        translated: nil,
        error_code: "requires_macos_15",
        error: "Apple Translate requires macOS 15 (Sequoia) or later."
    ))
}

#else
respond(TranslateResponse(
    translated: nil,
    error_code: "requires_macos_15",
    error: "Apple Translate requires macOS 15 (Sequoia) or later."
))
#endif
