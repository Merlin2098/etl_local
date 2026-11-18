// La CLI de Tauri busca su configuración en la carpeta actual; npm siempre corre los scripts en app/ui.
// Este envoltorio le indica dónde está app/src-tauri y que el frontend es app/ui.
import { fileURLToPath } from "node:url";
import { run } from "@tauri-apps/cli";

process.env.TAURI_APP_PATH ??= fileURLToPath(new URL("../../src-tauri", import.meta.url));
process.env.TAURI_FRONTEND_PATH ??= fileURLToPath(new URL("..", import.meta.url));

await run(process.argv.slice(2), "tauri");
