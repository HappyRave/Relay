// A node:test reporter for CI: each failing test becomes a GitHub Actions
// error annotation, shown on the run (and readable without opening the log).
import { relative } from "node:path";
import { fileURLToPath } from "node:url";

const escape = (s) => String(s).replace(/%/g, "%25").replace(/\r/g, "%0D").replace(/\n/g, "%0A");

export default async function* githubReporter(source) {
  for await (const event of source) {
    if (event.type !== "test:fail" || event.data.details?.type === "suite") continue;
    const { name, file, line, details } = event.data;
    const error = details?.error?.cause ?? details?.error;
    const message = error?.message ?? String(error);
    const path = file && relative(process.cwd(), file.startsWith("file:") ? fileURLToPath(file) : file).replaceAll("\\", "/");
    const where = path ? `file=${path},line=${line ?? 1},` : "";
    yield `::error ${where}title=${escape(name)}::${escape(message.slice(0, 2000))}\n`;
  }
}
