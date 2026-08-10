# Tags & Search

Free-form labels and key/value annotations to organize and search models.

## CLI

```bash
iv tag add my-llm production v2 stable
iv tag annotate my-llm --key team --value llm-platform
iv tag list my-llm
iv tag remove my-llm production

iv search llm
iv search "" --tag production
iv search llm --tag production --format json
```

## MCP tools

`tag_add`, `tag_remove`, `tag_list`, `tag_annotate`, `model_search`.

## REST

`/api/v1/models/{name}/tags`, `/api/v1/search`.

Tags are case-sensitive; search matches name substring AND every supplied tag/annotation filter. See [src/tags.rs](https://github.com/nervosys/IronVault/blob/master/src/tags.rs).
