use crate::error::AppError;

const MAX_WORD_LENGTH: usize = 128;

fn validate_word(word: &str) -> Result<String, AppError> {
    let trimmed = word.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_WORD_LENGTH
        || trimmed
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(AppError::validation(
            "Select one valid word to add to the dictionary.",
        ));
    }
    Ok(trimmed.to_owned())
}

#[cfg(windows)]
#[tauri::command]
pub fn add_word_to_native_dictionary(word: String, language: String) -> Result<(), AppError> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Globalization::{ISpellCheckerFactory, SpellCheckerFactory};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };

    let word = validate_word(&word)?;
    let language = language.trim();
    if language.is_empty() || language.len() > 35 || language.chars().any(char::is_whitespace) {
        return Err(AppError::validation(
            "Select a supported spellcheck language.",
        ));
    }

    unsafe {
        let init_result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let initialized_here = init_result.is_ok();
        let changed_mode = init_result.0 == 0x80010106u32 as i32;
        if init_result.is_err() && !changed_mode {
            return Err(AppError::io("Windows spellcheck could not be initialized."));
        }

        let result = (|| {
            let factory: ISpellCheckerFactory =
                CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER).map_err(
                    |error| {
                        log::error!("Windows spellcheck factory error: {error}");
                        AppError::io("Windows spellcheck is unavailable.")
                    },
                )?;
            let language_hstring = HSTRING::from(language);
            let language_pcwstr = PCWSTR(language_hstring.as_ptr());
            let supported = factory.IsSupported(language_pcwstr).map_err(|error| {
                log::error!("Windows spellcheck language check error: {error}");
                AppError::io("The selected Windows spellcheck language is unavailable.")
            })?;
            if !supported.as_bool() {
                return Err(AppError::validation(
                    "The selected spellcheck language is unavailable.",
                ));
            }
            let checker = factory
                .CreateSpellChecker(language_pcwstr)
                .map_err(|error| {
                    log::error!("Windows spellchecker creation error: {error}");
                    AppError::io("Windows spellcheck is unavailable.")
                })?;
            let word_hstring = HSTRING::from(word);
            checker.Add(&word_hstring).map_err(|error| {
                log::error!("Windows spellcheck add-word error: {error}");
                AppError::io("Windows could not add the word to the dictionary.")
            })
        })();

        if initialized_here {
            CoUninitialize();
        }
        result
    }
}

#[cfg(not(windows))]
#[tauri::command]
pub fn add_word_to_native_dictionary(_word: String, _language: String) -> Result<(), AppError> {
    Err(AppError::io(
        "Native dictionary editing is only available on Windows.",
    ))
}

#[cfg(test)]
mod tests {
    use super::{validate_word, MAX_WORD_LENGTH};
    use crate::error::ErrorCode;

    #[test]
    fn trims_and_accepts_one_word() {
        assert_eq!(validate_word("  JOI  ").unwrap(), "JOI");
    }

    #[test]
    fn rejects_empty_or_multi_word_input() {
        for value in ["", "   ", "two words", "line\nbreak"] {
            let error = validate_word(value).expect_err("invalid word should be rejected");
            assert_eq!(error.code, ErrorCode::Validation);
        }
    }

    #[test]
    fn rejects_words_longer_than_the_windows_limit() {
        let value = "a".repeat(MAX_WORD_LENGTH + 1);
        let error = validate_word(&value).expect_err("oversized word should be rejected");
        assert_eq!(error.code, ErrorCode::Validation);
    }

    #[cfg(not(windows))]
    #[test]
    fn reports_native_dictionary_as_windows_only() {
        let error = super::add_word_to_native_dictionary("JOI".into(), "en-US".into())
            .expect_err("non-Windows builds must not provide a native dictionary");
        assert_eq!(error.code, ErrorCode::Io);
    }
}
