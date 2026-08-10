# Model Validation

Integrity check — recomputes the SHA-256 of stored ciphertext and compares it against the version manifest. Catches silent corruption from bitrot, bad disks, or tampering.

## CLI

```bash
iv validate my-llm              # validate every version
iv validate my-llm --version 3  # one version
```

## MCP tool

`model_validate` — `{ "name": "...", "version": 3 }`

## REST

`POST /api/v1/models/{name}/validate`

## Output

```text
my-llm@1  OK   sha256=ab12…
my-llm@2  OK   sha256=cd34…
my-llm@3  FAIL expected=ef56… got=ee99…
```

Non-zero exit code on any failure. See [src/validation.rs](https://github.com/nervosys/IronVault/blob/master/src/validation.rs).
