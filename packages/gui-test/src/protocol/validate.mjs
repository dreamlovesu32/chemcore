import { readFile } from "node:fs/promises";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { schemaFiles } from "./paths.mjs";

const ajv = new Ajv2020({ allErrors: true, strict: true, strictRequired: false });
addFormats(ajv);
const validators = new Map();

async function validatorFor(schemaName) {
  if (!schemaFiles[schemaName]) {
    throw new Error(`Unsupported GUI test schema: ${schemaName || "<missing>"}`);
  }
  if (!validators.has(schemaName)) {
    validators.set(schemaName, readFile(schemaFiles[schemaName], "utf8").then((source) => {
      const schema = JSON.parse(source);
      return ajv.compile(schema);
    }));
  }
  return await validators.get(schemaName);
}

function formatErrors(errors = []) {
  return errors.map((error) => {
    const location = error.instancePath || "/";
    return `${location} ${error.message}`;
  });
}

export async function validateDocument(document) {
  const validator = await validatorFor(document?.schema);
  const valid = validator(document);
  return { valid, errors: valid ? [] : formatErrors(validator.errors) };
}

export async function assertValidDocument(document, label = "document") {
  const result = await validateDocument(document);
  if (!result.valid) {
    throw new Error(`${label} failed schema validation:\n${result.errors.map((error) => `- ${error}`).join("\n")}`);
  }
  return document;
}

export async function readValidatedDocument(path) {
  const document = JSON.parse(await readFile(path, "utf8"));
  return assertValidDocument(document, path);
}
