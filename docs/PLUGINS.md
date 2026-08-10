# Plugin System

Out-of-process extensions described by JSON manifests. Plugins surface their own CLI commands and event subscribers without recompiling `iv`.

## Manifest

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "0.1.0",
  "entrypoint": "./my-plugin.exe",
  "subscribes_to": ["ModelStored", "ModelDeleted"],
  "commands": [
    { "name": "hello", "description": "Say hi" }
  ]
}
```

## CLI

```bash
iv plugin discover                # scan well-known paths
iv plugin install ./manifest.json
iv plugin list
iv plugin info my-plugin
iv plugin uninstall my-plugin
```

## MCP tools

`plugin_discover`, `plugin_install`, `plugin_uninstall`, `plugin_list`, `plugin_info`.

Plugins run with the same user-account permissions as `iv` — only install plugins you trust. See [src/plugins.rs](https://github.com/nervosys/IronVault/blob/master/src/plugins.rs).
