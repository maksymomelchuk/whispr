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

    let semaphore = DispatchSemaphore(value: 0)

    Task {
        do {
            let availability = LanguageAvailability()
            let status = await availability.status(from: sourceLang, to: targetLang)
            switch status {
            case .installed:
                break
            case .supported:
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

            let session = TranslationSession(installedSource: sourceLang, target: targetLang)
            let result = try await session.translate(request.text)
            respond(TranslateResponse(translated: result.targetText, error_code: nil, error: nil))
        } catch {
            respond(TranslateResponse(
                translated: nil,
                error_code: "translation_failed",
                error: error.localizedDescription
            ))
        }
        semaphore.signal()
    }

    _ = semaphore.wait(timeout: .now() + 30)
    respond(TranslateResponse(
        translated: nil,
        error_code: "translation_failed",
        error: "Translation timed out."
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
