# Cross-Model Lineage DAG

While `iv lineage` shows the parent/child tree *within* one model, `iv lineage-graph` tracks derivations *across* models — e.g. `llama-base → llama-instruct → llama-quant`.

## Edge kinds

`fine-tune`, `distill`, `quantize`, `convert`, `merge`, `lora`.

## CLI

```bash
iv lineage-graph add --child llama-instruct --parents llama-base --kind fine-tune
iv lineage-graph add --child llama-q4 --parents llama-instruct --kind quantize
iv lineage-graph show
iv lineage-graph ancestors llama-q4
iv lineage-graph descendants llama-base
```

## MCP tools

`lineage_graph_add`, `lineage_graph_show`, `lineage_graph_ancestors`, `lineage_graph_descendants`.

The store is a DAG — cycles are rejected at insert time. See [src/lineage_graph.rs](https://github.com/nervosys/IronVault/blob/master/src/lineage_graph.rs).
