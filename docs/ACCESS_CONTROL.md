# Access Control (ACL)

Role-based ACLs per principal. Three built-in roles:

| Role     | Permissions                                              |
| -------- | -------------------------------------------------------- |
| `reader` | `list`, `get`, `versions`, `lineage`, `stats`            |
| `writer` | `reader` + `store`, `delete`, `convert`, `tag`, `policy` |
| `admin`  | everything, including `acl grant/revoke`                 |

## CLI

```bash
iv acl grant alice --role writer
iv acl check alice --role writer
iv acl list
iv acl revoke alice
```

## MCP tools

`acl_grant`, `acl_revoke`, `acl_list`, `acl_check`.

## REST

`/api/v1/acl` (list/grant/revoke).

JWT subject claims map directly to principal names — see the [OpenAPI specification](https://github.com/nervosys/IronVault/blob/master/.well-known/openapi.yaml) and [src/access_control.rs](https://github.com/nervosys/IronVault/blob/master/src/access_control.rs).
