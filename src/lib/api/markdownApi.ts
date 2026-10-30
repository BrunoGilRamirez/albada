import { invoke } from "@/lib/tauri";

export function addWordToNativeDictionary(word: string, language: string): Promise<void> {
  return invoke<void>("add_word_to_native_dictionary", { word, language });
}
