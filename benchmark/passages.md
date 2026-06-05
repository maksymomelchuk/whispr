# Speech engine benchmark passages

Read each passage **once, cleanly**, as you'd normally dictate. Save each as its own
clip in this folder using the suggested filename. Target format: **16 kHz, mono,
16-bit WAV** (matches the app pipeline). See recording instructions at the bottom.

Read verbatim — a misread inflates the error rate and isn't the model's fault. Don't
worry about punctuation; you speak the words, the model decides punctuation, and the
harness normalizes that out before scoring.

Each passage maps onto a Mode / use case:

| Passage | Stresses                             | Mode it informs                         |
| ------- | ------------------------------------ | --------------------------------------- |
| A       | Punctuation, contractions, names     | English dictation, cleaned-English      |
| B       | Technical jargon, keyterms, vocab    | Keyterm-heavy English dictation         |
| C       | Numbers, dates, units, identifiers   | English dictation formatting            |
| D       | Clean Ukrainian                      | Ukrainian                               |
| E       | Ukrainian + English code-switching   | Ukrainian robustness                    |
| F       | Translation quality (not WER-scored) | Ukrainian → English translate + cleanup |

---

## A — Everyday English → `a_english.wav`

> Hey Sarah, I wanted to follow up on yesterday's meeting. We've decided to push the launch to next Thursday, mostly because the design team isn't quite ready. Can you let Marcus know? I'll send over the updated roadmap this afternoon, and we should probably schedule a quick call before the weekend. Thanks so much — talk soon.

## B — Technical / jargon → `b_technical.wav`

> The Wispr Tauri app routes audio chunks through the Engine trait before handing them to Deepgram over a WebSocket. We refactored the async session loop so the i16 samples are buffered as sixteen kilohertz mono FLAC. ElevenLabs Scribe and the gpt-4o-transcribe model both run as batch POST requests, while Groq polls every three seconds.

## C — Numbers, dates, units, identifiers → `c_numbers.wav`

> The invoice came to twelve hundred fifty dollars and fifty cents, due on June fourth, twenty twenty-six. Our current version is one point eight point one. Please email the receipt to billing at serverless dot direct before three forty-five PM, and CC the finance team on the thread.

## D — Ukrainian → `d_ukrainian.wav`

> Привіт! Сьогодні я хочу розповісти про новий застосунок для розпізнавання мовлення. Він працює дуже швидко і підтримує кілька мов одночасно. Минулого тижня ми додали підтримку української, і тепер можна диктувати листи, нотатки та повідомлення майже без помилок.

## E — Ukrainian + English tech terms → `e_mixed.wav`

> Я щойно задеплоїв новий реліз через GitHub Actions, але білд впав на етапі signing. Здається, проблема в Windows сертифікаті. Давай зробимо rollback і перевіримо логи в CI перед тим, як мерджити пул реквест у main.

## F — Translation eyeball → `f_translate.wav`

> Доброго ранку! Дякую за вашу вчорашню допомогу з налаштуванням сервера. Усе працює ідеально, і команда дуже задоволена результатом.

_Expected English ≈ "Good morning! Thank you for your help yesterday with setting up the server. Everything works perfectly, and the team is very happy with the result."_

---

## Recording on macOS

Target format for every clip: **16 kHz, mono, 16-bit WAV.**

### Option 1 — Zero install (QuickTime + built-in `afconvert`)

1. QuickTime Player → File → New Audio Recording → record → stop → ⌘S (saves `.m4a`).
2. Convert each to WAV:

   ```sh
   afconvert -f WAVE -d LEI16@16000 -c 1 a_english.m4a a_english.wav
   ```

### Option 2 — CLI, record straight to WAV (`ffmpeg`)

```sh
brew install ffmpeg
ffmpeg -f avfoundation -list_devices true -i ""   # find your mic's index
ffmpeg -f avfoundation -i ":0" -ar 16000 -ac 1 a_english.wav   # press q to stop
```

Swap `:0` for your mic's index and the filename per passage.
