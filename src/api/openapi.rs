//! OpenAPI specification generator.

/// Minimal OpenAPI 3.0 spec for the vault API.
///
/// Returns a JSON-serialisable structure.  We generate the spec procedurally
/// rather than depending on a derive macro so the `api` feature stays light.
pub fn openapi_spec() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "IronVault API",
            "description": "RESTful API for secure AI model storage, versioning, and format conversion.",
            "version": "1.3.0",
            "license": {
                "name": "AGPL-3.0-or-later",
                "url": "https://www.gnu.org/licenses/agpl-3.0.html"
            }
        },
        "servers": [
            { "url": "/api/v1", "description": "Local server" }
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "tags": ["system"],
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/auth/token": {
                "post": {
                    "summary": "Obtain JWT token",
                    "tags": ["auth"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "passphrase": { "type": "string" }
                                    },
                                    "required": ["passphrase"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "JWT token" },
                        "401": { "description": "Invalid passphrase" }
                    }
                }
            },
            "/auth/logout": {
                "post": {
                    "summary": "Revoke the presented JWT",
                    "description": "Revokes the bearer token used to make this request so it \
                                    cannot be reused before its expiry. Revocations survive a \
                                    restart only when the server is started with a revocation \
                                    store; without one they are process-local, and they never \
                                    span replicas.",
                    "tags": ["auth"],
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Token revoked" },
                        "401": { "description": "Missing, malformed, or already invalid token" },
                        "500": { "description": "Revocation could not be persisted" }
                    }
                }
            },
            "/models": {
                "get": {
                    "summary": "List models in the vault",
                    "tags": ["models"],
                    "security": [{ "bearerAuth": [] }],
                    "responses": { "200": { "description": "Array of model names" } }
                }
            },
            "/models/{name}": {
                "get": {
                    "summary": "Retrieve latest model data",
                    "tags": ["models"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Model binary data" },
                        "404": { "description": "Model not found" }
                    }
                },
                "post": {
                    "summary": "Store a model (multipart upload)",
                    "tags": ["models"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string", "format": "binary" },
                                        "format": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["file", "format"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Version created" }
                    }
                }
            },
            "/models/{name}/versions": {
                "get": {
                    "summary": "List versions of a model",
                    "tags": ["versions"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "Array of versions" } }
                }
            },
            "/models/{name}/versions/{version}": {
                "get": {
                    "summary": "Retrieve specific model version data",
                    "tags": ["versions"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "version", "in": "path", "required": true, "schema": { "type": "integer" } }
                    ],
                    "responses": {
                        "200": { "description": "Model binary data" },
                        "404": { "description": "Version not found" }
                    }
                },
                "delete": {
                    "summary": "Delete a model version",
                    "tags": ["versions"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "version", "in": "path", "required": true, "schema": { "type": "integer" } }
                    ],
                    "responses": {
                        "200": { "description": "Deleted" },
                        "404": { "description": "Version not found" }
                    }
                }
            },
            "/models/{name}/lineage/{version}": {
                "get": {
                    "summary": "Get version lineage/history",
                    "tags": ["versions"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "version", "in": "path", "required": true, "schema": { "type": "integer" } }
                    ],
                    "responses": { "200": { "description": "Lineage array" } }
                }
            },
            "/conversions": {
                "get": {
                    "summary": "List supported format conversions",
                    "tags": ["conversions"],
                    "responses": { "200": { "description": "Conversion list" } }
                }
            },
            "/convert": {
                "post": {
                    "summary": "Convert model data between formats",
                    "tags": ["conversions"],
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data_base64": { "type": "string" },
                                        "source_format": { "type": "string" },
                                        "target_format": { "type": "string" },
                                        "quantization": { "type": "string" },
                                        "opset_version": { "type": "integer" },
                                        "validate": { "type": "boolean" }
                                    },
                                    "required": ["data_base64", "source_format", "target_format"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Conversion result. Check `converted` before using `data_base64`: when false, no conversion was performed, `data_base64` is absent, and `plan` describes the external tooling required.",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "converted": {
                                                "type": "boolean",
                                                "description": "True when data_base64 holds real target-format bytes."
                                            },
                                            "data_base64": {
                                                "type": "string",
                                                "description": "Converted model bytes. Absent when converted is false."
                                            },
                                            "plan": {
                                                "type": "object",
                                                "description": "Steps to perform this conversion with external tooling. Present only when converted is false."
                                            },
                                            "source_format": { "type": "string" },
                                            "target_format": { "type": "string" },
                                            "conversion_path": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            },
                                            "input_size": { "type": "integer" },
                                            "output_size": { "type": "integer" },
                                            "validation": { "type": "object", "nullable": true }
                                        },
                                        "required": ["converted", "source_format", "target_format", "conversion_path", "input_size", "output_size"]
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/models/{name}/card": {
                "get": {
                    "summary": "Generate model card from vault metadata",
                    "tags": ["model-cards"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Model card JSON" },
                        "404": { "description": "Model not found" }
                    }
                },
                "post": {
                    "summary": "Create or overwrite a custom model card",
                    "tags": ["model-cards"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "name", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "model_details": { "type": "object" },
                                        "intended_use": { "type": "object" },
                                        "metadata": { "type": "object" },
                                        "created_at": { "type": "string", "format": "date-time" },
                                        "updated_at": { "type": "string", "format": "date-time" }
                                    },
                                    "required": ["model_details", "intended_use"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Model card created" },
                        "400": { "description": "Invalid model card JSON" },
                        "404": { "description": "Model not found" }
                    }
                }
            },
            "/compliance": {
                "get": {
                    "summary": "Run FIPS 140-3, CVE, MITRE ATT&CK, and CMMC 2.0 compliance checks",
                    "tags": ["compliance"],
                    "security": [{ "bearerAuth": [] }],
                    "responses": { "200": { "description": "Compliance report" } }
                }
            },
            "/rag/search": {
                "post": {
                    "summary": "Search RAG knowledge base",
                    "tags": ["rag"],
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "query": { "type": "string" },
                                        "limit": { "type": "integer" }
                                    },
                                    "required": ["query"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Search results" },
                        "400": { "description": "Empty query" }
                    }
                }
            },
            "/rag/documents": {
                "post": {
                    "summary": "Add document to RAG knowledge base",
                    "tags": ["rag"],
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "content": { "type": "string" },
                                        "metadata": { "type": "object" }
                                    },
                                    "required": ["content"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Document stored with ID and content_length" },
                        "400": { "description": "Empty content" }
                    }
                }
            },
            "/metrics": {
                "get": {
                    "summary": "Prometheus-compatible metrics",
                    "tags": ["system"],
                    "security": [{ "bearerAuth": [] }],
                    "responses": { "200": { "description": "Metrics in Prometheus text format" } }
                }
            },
            "/events": {
                "get": {
                    "summary": "Event stream (recent audit events)",
                    "description": "Returns recent events. Non-admin roles have security events filtered out.",
                    "tags": ["system"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "limit", "in": "query", "schema": { "type": "integer" } }
                    ],
                    "responses": { "200": { "description": "Event entries" } }
                }
            },
            "/stats": {
                "get": {
                    "summary": "Vault statistics",
                    "tags": ["system"],
                    "security": [{ "bearerAuth": [] }],
                    "responses": { "200": { "description": "Vault stats" } }
                }
            },
            "/audit": {
                "get": {
                    "summary": "Audit log entries",
                    "description": "Returns audit log. Non-admin roles have security events filtered out.",
                    "tags": ["system"],
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        { "name": "limit", "in": "query", "schema": { "type": "integer" } }
                    ],
                    "responses": { "200": { "description": "Audit entries" } }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        }
    })
}
