import { readFileSync } from "node:fs";
import { join } from "node:path";

const file = readFileSync(join(import.meta.dirname, "test_file.txt"), {
  encoding: "utf-8",
});

console.log(`Uploading file test_file.txt with content: ${file}`);

const response = await fetch("http://0.0.0.0:5558/upload", {
  method: "POST",
  body: file,
});

if (response.ok) {
  console.log("File uploaded successfully");
} else {
  console.error(`Failed to upload file. Status code: ${response.status}`);
  process.exit(1);
}

const bundleId = await response.text();
console.log(`Bundle ID: ${bundleId}`);
console.log(
  `Run "BUNDLE_ID=${bundleId} node download.js" to download the file`
);
