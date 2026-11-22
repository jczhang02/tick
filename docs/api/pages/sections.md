# Sections

## Endpoints

### Search Sections

`GET /api/v1/sections/search`

Search active sections by name, optionally filtered by project or workspace.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "query",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "minLength": 1,
        "maxLength": 1024,
        "description": "Search query to match section names. Matching is case-insensitive. Queries are matched literally unless `*` (wildcard) is included. Use `\\*` for literal asterisk and `\\\\` for literal backslash.",
        "examples": [
          "To Do",
          "Week *",
          "Q* 2026",
          "Draft\\*"
        ],
        "title": "Query"
      }
    },
    {
      "name": "project_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "String ID of the project to search sections from. If omitted or null, search sections from all projects. If provided, workspace_id is ignored.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "exclusiveMinimum": 0,
            "description": "Numeric ID of the workspace.",
            "examples": [
              123456
            ]
          },
          {
            "type": "null"
          }
        ],
        "description": "Numeric ID of the workspace to search sections from, limited to the user's joined projects. If omitted or null, search sections from all projects. Ignored if project_id is also provided.",
        "title": "Workspace Id"
      }
    },
    {
      "name": "cursor",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "minLength": 1,
            "pattern": "^[0-9a-zA-Z_-]+\\.[0-9a-zA-Z_-]+$",
            "description": "An opaque string used as the cursor for pagination. Must be used with the same parameters from the previous request",
            "examples": [
              "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
            ]
          },
          {
            "type": "null"
          }
        ],
        "title": "Cursor"
      }
    },
    {
      "name": "limit",
      "in": "query",
      "required": false,
      "schema": {
        "type": "integer",
        "maximum": 200,
        "exclusiveMinimum": 0,
        "description": "The number of objects to return in a page",
        "examples": [
          50
        ],
        "default": 50,
        "title": "Limit"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PaginatedList_SectionSyncView_"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Unarchive Section

`POST /api/v1/sections/{section_id}/unarchive`

Marks a section as active again.

```json
{
  "parameters": [
    {
      "name": "section_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the section",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SectionSyncView"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Archive Section

`POST /api/v1/sections/{section_id}/archive`

Marks a section as archived.

```json
{
  "parameters": [
    {
      "name": "section_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the section",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SectionSyncView"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Delete Section

`DELETE /api/v1/sections/{section_id}`

Delete the section and all of its tasks

```json
{
  "parameters": [
    {
      "name": "section_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the section",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    }
  ],
  "responses": {
    "204": {
      "description": "Successful Response"
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Get Section

`GET /api/v1/sections/{section_id}`

Return the section for the given section ID

```json
{
  "parameters": [
    {
      "name": "section_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the section",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    },
    {
      "name": "public_key",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "description": "Public project access key.",
            "examples": [
              "550e8400-e29b-41d4-a716-446655440000"
            ]
          },
          {
            "type": "null"
          }
        ],
        "title": "Public Key"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SectionSyncView"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Update Section

`POST /api/v1/sections/{section_id}`

Update a section.

```json
{
  "parameters": [
    {
      "name": "section_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the section",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_163067b4"
        }
      }
    }
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SectionSyncView"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Create Section

`POST /api/v1/sections`

Create a new section

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_134c19e9"
        }
      }
    }
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SectionSyncView"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Get Sections

`GET /api/v1/sections`

Get all active sections for the user, optionally filtered by project or workspace.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "String ID of the project to get sections from. If omitted or null, get sections from all projects. If provided, workspace_id is ignored.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "exclusiveMinimum": 0,
            "description": "Numeric ID of the workspace.",
            "examples": [
              123456
            ]
          },
          {
            "type": "null"
          }
        ],
        "description": "Numeric ID of the workspace to get sections from, limited to the user's joined projects. If omitted or null, get sections from all projects. Ignored if project_id is also provided.",
        "title": "Workspace Id"
      }
    },
    {
      "name": "cursor",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "minLength": 1,
            "pattern": "^[0-9a-zA-Z_-]+\\.[0-9a-zA-Z_-]+$",
            "description": "An opaque string used as the cursor for pagination. Must be used with the same parameters from the previous request",
            "examples": [
              "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
            ]
          },
          {
            "type": "null"
          }
        ],
        "title": "Cursor"
      }
    },
    {
      "name": "limit",
      "in": "query",
      "required": false,
      "schema": {
        "type": "integer",
        "maximum": 200,
        "exclusiveMinimum": 0,
        "description": "The number of objects to return in a page",
        "examples": [
          50
        ],
        "default": 50,
        "title": "Limit"
      }
    },
    {
      "name": "public_key",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "description": "Public project access key.",
            "examples": [
              "550e8400-e29b-41d4-a716-446655440000"
            ]
          },
          {
            "type": "null"
          }
        ],
        "title": "Public Key"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PaginatedList_SectionSyncView_"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

## Schemas

The endpoints above refer to these schemas with `$ref`.

### Body_134c19e9

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the new section.",
      "examples": [
        "Groceries"
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "ID of the project to add the section to",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "order": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Order",
      "description": "Position of the new section in the project",
      "examples": [
        12
      ]
    },
    "description": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Description",
      "description": "Description of the new section.",
      "examples": [
        "Tasks for the launch plan"
      ]
    }
  },
  "type": "object",
  "required": [
    "name",
    "project_id"
  ],
  "title": "Body"
}
```

### Body_163067b4

```json
{
  "properties": {
    "name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Name",
      "description": "Updated section name. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Groceries"
      ]
    },
    "section_order": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 2147483647.0,
          "minimum": -2147483648.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Section Order",
      "description": "Updated position of the section in the project. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        12
      ]
    },
    "is_collapsed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Collapsed",
      "description": "Updated collapsed state of the section. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        true
      ]
    },
    "description": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Description",
      "description": "Updated section description. Pass null to clear the value. Omit this field to keep it unchanged.",
      "examples": [
        "Tasks for the launch plan"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### PaginatedList_SectionSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Results",
      "description": "Objects in the current page of results."
    },
    "next_cursor": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Next Cursor",
      "description": "Cursor for the next page of results, or `null` when there are no more results.",
      "examples": [
        "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
      ]
    }
  },
  "type": "object",
  "required": [
    "results",
    "next_cursor"
  ],
  "title": "PaginatedList"
}
```

### SectionSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the section.",
      "examples": [
        "6fFPHV272WWh3gpW"
      ]
    },
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "String ID of the user who owns the section.",
      "examples": [
        "1234567"
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "String ID of the project that contains the section.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "added_at": {
      "type": "string",
      "format": "date-time",
      "title": "Added At",
      "description": "Date and time when the section was created.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "updated_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Updated At",
      "description": "Date and time when the section was last updated, or `null` if unknown.",
      "examples": [
        "2025-01-16T10:30:00Z"
      ]
    },
    "archived_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Archived At",
      "description": "Date and time when the section was archived, or `null` if it is active.",
      "examples": [
        "2025-01-17T10:30:00Z"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Section name.",
      "examples": [
        "Groceries"
      ]
    },
    "description": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Description",
      "description": "Section description, or `null` if it has none.",
      "examples": [
        "Tasks for the launch plan"
      ]
    },
    "section_order": {
      "type": "integer",
      "title": "Section Order",
      "description": "Position of the section in the project.",
      "examples": [
        1
      ]
    },
    "order_key": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Order Key",
      "description": "Fractional-indexing order key: sections sort by comparing keys lexicographically within a project. May be `null` for projects not yet migrated.",
      "examples": [
        "a1V"
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Whether the section is collapsed in the user's view.",
      "examples": [
        false
      ]
    },
    "is_archived": {
      "type": "boolean",
      "title": "Is Archived",
      "description": "Whether the section is archived.",
      "examples": [
        false
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the section is deleted.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "user_id",
    "project_id",
    "added_at",
    "updated_at",
    "archived_at",
    "name",
    "description",
    "section_order",
    "order_key",
    "is_collapsed",
    "is_archived",
    "is_deleted"
  ],
  "title": "SectionSyncView",
  "description": "A class with fields representing a SectionView which will be returned\nto clients in a sync (or sync-like) response."
}
```
