import js from "@eslint/js";
import typescript from "typescript-eslint";

export default typescript.config(
  {
    ignores: ["dist/", "node_modules/", "src-tauri/target/"],
  },
  js.configs.recommended,
  ...typescript.configs.recommended,
);
