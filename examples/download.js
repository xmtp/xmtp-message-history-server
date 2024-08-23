const BUNDLE_ID = process.env.BUNDLE_ID;

if (!BUNDLE_ID) {
  console.error("BUNDLE_ID must be present in the environment");
  process.exit(1);
}

console.log(`Downloading bundle: ${BUNDLE_ID}`);

const url = `http://0.0.0.0:5558/files/${BUNDLE_ID}`;

const response = await fetch(url);

if (!response.ok) {
  console.error(`Failed to download bundle. Status code: ${response.status}`);
  process.exit(1);
}

const text = await response.text();

console.log(`Downloaded bundle with content: ${text}`);
