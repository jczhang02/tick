# Projects

## Endpoints

### Create Project From Template

`POST /api/v1/projects/from_template`

Create a project from a template, returning only its ID and success status.

Requires `data:read_write`. Setup templates are not supported. Workspace
templates can only be imported into their associated workspace. Calendar
defaults are applied to the project according to the destination plan.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_88fd4a9a"
        }
      }
    },
    "required": true
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/ProjectImportCreateResponse"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate limiter temporarily unavailable"
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

### Permissions

`GET /api/v1/projects/permissions`

Returns a list of all the available roles and the associated actions they can
perform in a project.

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/RoleToActionMappingView"
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

### Get Archived

`GET /api/v1/projects/archived`

Get the user's archived projects.

```json
{
  "parameters": [
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
            "$ref": "#/components/schemas/PaginatedList_AnyProjectSyncViewResponse_"
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

### Create Project From Csv

`POST /api/v1/projects/import`

Create a project from CSV and return its ID and imported objects.

Send a multipart file upload or raw CSV request body. For raw CSV, pass name
and optional workspace_id as query parameters. Invalid UTF-8 bytes are replaced.

```json
{
  "parameters": [
    {
      "name": "name",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads. Required for raw uploads.",
      "schema": {
        "type": "string",
        "title": "Name",
        "description": "Name of the new project.",
        "examples": [
          "My Project"
        ]
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads.",
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "title": "Workspace Id",
        "description": "Workspace ID for the new project. Omit to create a personal project.",
        "examples": [
          "123456"
        ]
      }
    }
  ],
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_create_project_from_csv_api_v1_projects_import_post"
        }
      },
      "text/csv": {
        "schema": {
          "type": "string",
          "format": "binary"
        }
      }
    },
    "required": true
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/ProjectImportCreateResponseWithObjects"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate limiter unavailable"
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

```curl

$ curl https://api.todoist.com/api/v1/projects/import \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -F "name=My Project" \
       -F file=@example.csv
```

### Search Projects

`GET /api/v1/projects/search`

Search active user projects by name.

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
        "description": "Search query to match project names. Matching is case-insensitive. Queries are matched literally unless `*` (wildcard) is included. Use `\\*` for literal asterisk and `\\\\` for literal backslash.",
        "examples": [
          "Inbox",
          "Client *",
          "Q* 2026",
          "Draft\\*"
        ],
        "title": "Query"
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
            "$ref": "#/components/schemas/PaginatedList_AnyProjectSyncViewResponse_"
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

### Import Project Template

`POST /api/v1/projects/{project_id}/import_template`

Import a project template, returning tasks, comments, reminders and applied defaults.

Requires `data:read_write` and permission to apply templates to the project.
The project must be active and unfrozen. Workspace templates can only be
imported into their associated workspace. Setup templates are rejected; use
`/templates/{template_id}/use`. The `projects` array is empty.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "ID of the project to import into.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_9659a42d"
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
            "$ref": "#/components/schemas/ProjectImportResponse"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate limiter temporarily unavailable"
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

### Get Project Collaborators

`GET /api/v1/projects/{project_id}/collaborators`

Get all collaborators for a given project.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
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
            "$ref": "#/components/schemas/PaginatedList_Collaborator_"
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

### Unarchive Project

`POST /api/v1/projects/{project_id}/unarchive`

Marks a previously archived project as active again. For personal projects, this
will make the project visible again for the initiating user. For workspace projects,
this will make the project visible again for all applicable workspace users.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### Archive Project

`POST /api/v1/projects/{project_id}/archive`

Marks a project as archived. For personal projects, this will archive it just for
the initiating user (leaving it visible to any other collaborators). For workspace
projects, this will archive it for all workspace users, removing it from view.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### Import Project Csv

`POST /api/v1/projects/{project_id}/import`

Import CSV into an existing project and return the imported objects.

Send a multipart file upload or raw CSV request body. CSV is decoded as UTF-8,
replacing invalid bytes, and must include TYPE and CONTENT header columns.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "ID of the project to import the CSV into.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_import_project_csv_api_v1_projects__project_id__import_post"
        }
      },
      "text/csv": {
        "schema": {
          "type": "string",
          "format": "binary"
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
            "$ref": "#/components/schemas/ProjectImportResponse"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate limiter unavailable"
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

```curl

$ curl https://api.todoist.com/api/v1/projects/6XGgm6PHrGgMpCFX/import \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -F file=@example.csv
```

### Join

`POST /api/v1/projects/{project_id}/join`

_Only used for workspaces_

This endpoint is used to join a workspace project by a workspace_user and
is only usable by the workspace user.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the workspace project to join.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_GetDataV2Response"
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

### Delete Project

`DELETE /api/v1/projects/{project_id}`

Deletes a project and all of its sections and tasks.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
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

### Get Project

`GET /api/v1/projects/{project_id}`

Returns a project object related to the given ID

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### Update Project

`POST /api/v1/projects/{project_id}`

Updates a project and returns it.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_cb8d28ec"
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
            "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### Create Project

`POST /api/v1/projects`

Creates a new project and returns it

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_207bf8bc"
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
            "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### Get Projects

`GET /api/v1/projects`

Get all active user projects, optionally filtered by folder or workspace.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "folder_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "exclusiveMinimum": 0,
            "description": "Numeric ID of the folder.",
            "examples": [
              1234
            ]
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter projects to only those in this folder. If provided, workspace_id is ignored.",
        "title": "Folder Id"
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
        "description": "Filter projects to only those in this workspace. Ignored if folder_id is also provided.",
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
            "$ref": "#/components/schemas/PaginatedList_AnyProjectSyncViewResponse_"
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

### APIColorID

```json
{
  "anyOf": [
    {
      "type": "string",
      "enum": [
        "berry_red",
        "red",
        "orange",
        "yellow",
        "olive_green",
        "lime_green",
        "green",
        "mint_green",
        "teal",
        "sky_blue",
        "light_blue",
        "blue",
        "grape",
        "violet",
        "lavender",
        "magenta",
        "salmon",
        "charcoal",
        "grey",
        "taupe"
      ]
    },
    {
      "type": "integer",
      "enum": [
        30,
        31,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        42,
        43,
        44,
        45,
        46,
        47,
        48,
        49
      ]
    }
  ],
  "examples": [
    "charcoal"
  ]
}
```

### ActionView

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Permission action name.",
      "examples": [
        "UPDATE_ITEM"
      ]
    }
  },
  "type": "object",
  "required": [
    "name"
  ],
  "title": "ActionView"
}
```

### AnyProjectSyncViewResponse

```json
{
  "anyOf": [
    {
      "$ref": "#/components/schemas/PersonalProjectSyncView"
    },
    {
      "$ref": "#/components/schemas/WorkspaceProjectSyncView"
    }
  ],
  "title": "Project object",
  "description": "Can be either a personal or a workspace project."
}
```

### Body_207bf8bc

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
      "description": "Name of the project.",
      "examples": [
        "Vacation Planning"
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
      "description": "Description of the project.",
      "examples": [
        "Tasks for planning the trip."
      ]
    },
    "parent_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Parent Id",
      "description": "Parent project ID. If provided, creates this project as a sub-project",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "color": {
      "$ref": "#/components/schemas/APIColorID",
      "description": "Color of the project icon.",
      "default": "charcoal"
    },
    "icon": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "briefcase",
            "folder",
            "diagram",
            "headset",
            "microphone",
            "flag",
            "bullseye",
            "house",
            "heart",
            "airplane",
            "gift",
            "pin",
            "school_cap",
            "lightbulb",
            "star",
            "command_prompt",
            "laptop",
            "envelope",
            "calendar",
            "person",
            "sofa",
            "cup_and_heat_waves",
            "paintpalette",
            "broom",
            "sun_max",
            "moon",
            "beach_umbrella",
            "pizza",
            "checkmark_line_3",
            "basketball",
            "face_smiling",
            "dog",
            "dollarsign",
            "smartphone",
            "person_2",
            "chart_line_uptrend_xyaxis_in_square",
            "laboratory_flask",
            "telescope",
            "book",
            "document",
            "sparkles",
            "video",
            "paintbrush",
            "puzzlepiece_extension",
            "clock",
            "pills",
            "birthday_cake",
            "stethoscope",
            "apple"
          ],
          "examples": [
            "lightbulb"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Icon",
      "description": "Project icon name. Pass null or omit it for the default icon.",
      "examples": [
        "lightbulb"
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the project is a favorite for the user.",
      "default": false,
      "examples": [
        false
      ]
    },
    "view_style": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectViewStyleV1"
        },
        {
          "type": "null"
        }
      ],
      "description": "View style of the project.",
      "examples": [
        "list"
      ]
    },
    "workspace_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Workspace Id",
      "description": "Workspace ID. If provided, creates a workspace project instead of a personal project.",
      "examples": [
        123456
      ]
    }
  },
  "type": "object",
  "required": [
    "name"
  ],
  "title": "Body"
}
```

### Body_88fd4a9a

```json
{
  "properties": {
    "template_id": {
      "type": "string",
      "title": "Template Id",
      "description": "Curated template slug or saved template ID with a `UT_` prefix.",
      "examples": [
        "reading-list"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Project name. Empty or omitted uses the localized template name.",
      "default": "",
      "examples": [
        "Reading list"
      ]
    },
    "locale": {
      "type": "string",
      "maxLength": 10,
      "title": "Locale",
      "description": "Locale for curated template content, with the template's English fallback.",
      "default": "en",
      "examples": [
        "en"
      ]
    },
    "workspace_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Workspace Id",
      "description": "Destination workspace ID. Omit to create a personal project.",
      "examples": [
        "123456"
      ]
    },
    "folder_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Folder Id",
      "description": "Destination folder ID. Requires workspace_id and a folder in that workspace.",
      "examples": [
        "123456"
      ]
    }
  },
  "type": "object",
  "required": [
    "template_id"
  ],
  "title": "Body"
}
```

### Body_9659a42d

```json
{
  "properties": {
    "template_id": {
      "type": "string",
      "title": "Template Id",
      "description": "Curated template slug or saved template ID with a `UT_` prefix.",
      "examples": [
        "reading-list"
      ]
    },
    "locale": {
      "type": "string",
      "maxLength": 10,
      "title": "Locale",
      "description": "Locale for curated template content, with the template's English fallback.",
      "default": "en",
      "examples": [
        "en"
      ]
    }
  },
  "type": "object",
  "required": [
    "template_id"
  ],
  "title": "Body"
}
```

### Body_cb8d28ec

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
      "description": "Updated project name. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Vacation Planning"
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
      "description": "Updated project description. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Tasks for planning the trip."
      ]
    },
    "color": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/APIColorID"
        },
        {
          "type": "null"
        }
      ],
      "description": "Updated project color. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "charcoal"
      ]
    },
    "icon": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "briefcase",
            "folder",
            "diagram",
            "headset",
            "microphone",
            "flag",
            "bullseye",
            "house",
            "heart",
            "airplane",
            "gift",
            "pin",
            "school_cap",
            "lightbulb",
            "star",
            "command_prompt",
            "laptop",
            "envelope",
            "calendar",
            "person",
            "sofa",
            "cup_and_heat_waves",
            "paintpalette",
            "broom",
            "sun_max",
            "moon",
            "beach_umbrella",
            "pizza",
            "checkmark_line_3",
            "basketball",
            "face_smiling",
            "dog",
            "dollarsign",
            "smartphone",
            "person_2",
            "chart_line_uptrend_xyaxis_in_square",
            "laboratory_flask",
            "telescope",
            "book",
            "document",
            "sparkles",
            "video",
            "paintbrush",
            "puzzlepiece_extension",
            "clock",
            "pills",
            "birthday_cake",
            "stethoscope",
            "apple"
          ],
          "examples": [
            "lightbulb"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Icon",
      "description": "Updated project icon name. Pass null to reset to the default icon. Omit this field to keep it unchanged.",
      "examples": [
        "lightbulb"
      ]
    },
    "is_favorite": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Favorite",
      "description": "Whether the project is marked as a favorite. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        true
      ]
    },
    "view_style": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectViewStyleV1"
        },
        {
          "type": "null"
        }
      ],
      "description": "Updated project view style. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "board"
      ]
    },
    "child_order": {
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
      "title": "Child Order",
      "description": "Updated position of the project in the hierarchy. Only supported for personal projects. Passing null or omitting this field will leave it unchanged.",
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
      "description": "Whether the project is collapsed. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        false
      ]
    },
    "folder_id": {
      "anyOf": [
        {
          "type": "integer",
          "exclusiveMinimum": 0.0,
          "description": "Numeric ID of the folder.",
          "examples": [
            1234
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Folder Id",
      "description": "Folder to move the project into. Only supported for workspace projects. Pass null to clear the value. Omit this field to keep it unchanged."
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_create_project_from_csv_api_v1_projects_import_post

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the new project.",
      "examples": [
        "My Project"
      ]
    },
    "workspace_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Workspace Id",
      "description": "Workspace ID for the new project. Omit to create a personal project.",
      "examples": [
        "123456"
      ]
    },
    "file": {
      "type": "string",
      "contentMediaType": "application/octet-stream",
      "title": "File",
      "description": "The file to upload."
    }
  },
  "type": "object",
  "required": [
    "name",
    "file"
  ],
  "title": "Body_create_project_from_csv_api_v1_projects_import_post"
}
```

### Body_import_project_csv_api_v1_projects__project_id__import_post

```json
{
  "properties": {
    "file": {
      "type": "string",
      "contentMediaType": "application/octet-stream",
      "title": "File",
      "description": "The file to upload."
    }
  },
  "type": "object",
  "required": [
    "file"
  ],
  "title": "Body_import_project_csv_api_v1_projects__project_id__import_post"
}
```

### CalendarColor

```json
{
  "type": "string",
  "enum": [
    "PRIORITY",
    "LABEL",
    "PROJECT"
  ],
  "title": "CalendarColor",
  "x-enumDescriptions": {
    "LABEL": "Label",
    "PRIORITY": "Priority",
    "PROJECT": "Project"
  }
}
```

### CalendarLayout

```json
{
  "type": "string",
  "enum": [
    "WEEK",
    "MONTH"
  ],
  "title": "CalendarLayout",
  "x-enumDescriptions": {
    "MONTH": "Month",
    "WEEK": "Week"
  }
}
```

### CalendarSettings

```json
{
  "properties": {
    "layout": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CalendarLayout"
        },
        {
          "type": "null"
        }
      ],
      "description": "Calendar layout: `WEEK` or `MONTH`.",
      "default": "WEEK",
      "examples": [
        "WEEK"
      ]
    },
    "visible_day_count": {
      "anyOf": [
        {
          "type": "integer",
          "enum": [
            1,
            3,
            7
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Visible Day Count",
      "description": "Number of consecutive days shown in the week calendar layout.",
      "examples": [
        1,
        3,
        7
      ]
    },
    "color": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CalendarColor"
        },
        {
          "type": "null"
        }
      ],
      "description": "Attribute used to color tasks in calendar layouts. Defaults to priority when omitted.",
      "examples": [
        "PRIORITY",
        "LABEL",
        "PROJECT"
      ]
    }
  },
  "type": "object",
  "title": "CalendarSettings"
}
```

### Collaborator

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "The user's ID",
      "examples": [
        "0192837465"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "The user's full name",
      "examples": [
        "John Smith"
      ]
    },
    "email": {
      "type": "string",
      "title": "Email",
      "description": "The user's email address",
      "examples": [
        "email@example.org"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "email"
  ],
  "title": "Collaborator"
}
```

### CollaboratorRole

```json
{
  "type": "string",
  "enum": [
    "CREATOR",
    "ADMIN",
    "READ_WRITE",
    "EDIT_ONLY",
    "COMPLETE_ONLY"
  ],
  "title": "CollaboratorRole",
  "description": "User role in the project. For personal project the role should be always \"CREATOR\"\nUser role for projects v1 maybe specified as \"CREATOR\" or \"ADMIN\", because in the\npast there was no permissions.",
  "x-enumDescriptions": {
    "ADMIN": "Admin",
    "COMPLETE_ONLY": "Complete Only",
    "CREATOR": "Creator",
    "EDIT_ONLY": "Edit Only",
    "READ_WRITE": "Read Write"
  }
}
```

### DiscardedField

```json
{
  "properties": {
    "field": {
      "$ref": "#/components/schemas/DiscardedFieldName",
      "description": "The task property that was dropped.",
      "examples": [
        "duration"
      ]
    },
    "reason": {
      "$ref": "#/components/schemas/DiscardedFieldReason",
      "description": "Which plan excluded the property, and so which one has to change to keep it.",
      "examples": [
        "workspace_plan_limit"
      ]
    }
  },
  "type": "object",
  "required": [
    "field",
    "reason"
  ],
  "title": "DiscardedField",
  "description": "A task property the importer dropped, and the plan that excluded it."
}
```

### DiscardedFieldName

```json
{
  "type": "string",
  "enum": [
    "deadline",
    "duration"
  ],
  "title": "DiscardedFieldName",
  "description": "A task property the importer can drop because the plan excludes it.",
  "x-enumDescriptions": {
    "deadline": "Deadline",
    "duration": "Duration"
  }
}
```

### DiscardedFieldReason

```json
{
  "type": "string",
  "enum": [
    "personal_plan_limit",
    "workspace_plan_limit"
  ],
  "title": "DiscardedFieldReason",
  "description": "Which plan excluded the field, and so which one has to change to keep it:\n`workspace_plan_limit` the workspace the target project belongs to,\n`personal_plan_limit` the importing user's own subscription.",
  "x-enumDescriptions": {
    "personal_plan_limit": "Personal Plan Limit",
    "workspace_plan_limit": "Workspace Plan Limit"
  }
}
```

### EffectScope

```json
{
  "properties": {
    "mode": {
      "type": "string",
      "const": "all",
      "title": "Mode",
      "description": "Where the effect applies. Always `all` for now.",
      "default": "all"
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "EffectScope"
}
```

### ExposedCollaborator

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the collaborator.",
      "examples": [
        "123456"
      ]
    },
    "full_name": {
      "type": "string",
      "title": "Full Name",
      "description": "Full name of the collaborator.",
      "examples": [
        "Ada Lovelace"
      ]
    },
    "email": {
      "type": "string",
      "title": "Email",
      "description": "Email address of the collaborator.",
      "examples": [
        "ada@example.com"
      ]
    },
    "timezone": {
      "type": "string",
      "title": "Timezone",
      "description": "Timezone of the collaborator.",
      "examples": [
        "Europe/London"
      ]
    },
    "image_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Image Id",
      "description": "Avatar image ID for the collaborator.",
      "examples": [
        null
      ]
    },
    "status": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/UserStatus"
        },
        {
          "type": "null"
        }
      ],
      "description": "The collaborator's status, shared across Todoist and Comms. `null` when they have none or it has expired.",
      "examples": [
        null
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the collaborator has been deleted.",
      "default": false,
      "examples": [
        false
      ]
    },
    "is_agent": {
      "type": "boolean",
      "title": "Is Agent",
      "description": "Whether the collaborator is an agent, such as `@todoist`, rather than a person. Only present when `true`.",
      "default": false,
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "full_name",
    "email",
    "timezone",
    "image_id"
  ],
  "title": "ExposedCollaborator",
  "description": "A project collaborator, as exposed in sync and REST API responses."
}
```

### FolderView

```json
{
  "properties": {
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "String ID of the workspace that contains the folder.",
      "examples": [
        "123456"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Folder name.",
      "default": "",
      "examples": [
        "Design"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the folder is deleted.",
      "default": false,
      "examples": [
        false
      ]
    },
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the folder.",
      "default": "0",
      "examples": [
        "1234"
      ]
    },
    "default_order": {
      "type": "integer",
      "title": "Default Order",
      "description": "Default position of the folder in the workspace.",
      "default": 0,
      "examples": [
        1
      ]
    },
    "default_order_key": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Default Order Key",
      "description": "Fractional-indexing key for the workspace's shared default-ordering scope (folders and workspace projects together). `null` until assigned (backfill pending).",
      "examples": [
        "a1V"
      ]
    },
    "child_order": {
      "type": "integer",
      "title": "Child Order",
      "description": "User-specific position of the folder.",
      "default": 0,
      "examples": [
        1
      ]
    }
  },
  "type": "object",
  "required": [
    "workspace_id"
  ],
  "title": "FolderView"
}
```

### ImportedProjectViewOptionsDefault

```json
{
  "properties": {
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "ID of the project whose defaults were applied.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "view_mode": {
      "$ref": "#/components/schemas/ViewMode",
      "description": "Default view mode applied to the project.",
      "examples": [
        "CALENDAR"
      ]
    },
    "calendar_settings": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CalendarSettings"
        },
        {
          "type": "null"
        }
      ],
      "description": "Calendar settings applied to the project, or null when absent.",
      "examples": [
        {
          "layout": "MONTH"
        }
      ]
    }
  },
  "type": "object",
  "required": [
    "project_id",
    "view_mode",
    "calendar_settings"
  ],
  "title": "ImportedProjectViewOptionsDefault",
  "description": "Calendar-related subset of the defaults persisted for an imported project."
}
```

### ItemNoteSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the comment.",
      "examples": [
        "6XGgmFQrx44wfGHr"
      ]
    },
    "posted_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted Uid",
      "description": "String ID of the user who posted the comment.",
      "examples": [
        "1234567"
      ]
    },
    "content": {
      "type": "string",
      "title": "Content",
      "description": "Comment content.",
      "default": "",
      "examples": [
        "Looks good to me."
      ]
    },
    "file_attachment": {
      "anyOf": [
        {
          "additionalProperties": {
            "anyOf": [
              {
                "type": "string"
              },
              {
                "type": "integer"
              },
              {
                "items": {},
                "type": "array"
              },
              {
                "additionalProperties": true,
                "type": "object"
              },
              {
                "type": "null"
              }
            ]
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Attachment",
      "description": "File attachment metadata, or `null` when the comment has no attachment.",
      "examples": [
        null
      ]
    },
    "uids_to_notify": {
      "anyOf": [
        {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        {
          "type": "null"
        }
      ],
      "title": "Uids To Notify",
      "description": "String IDs of users notified by the comment.",
      "examples": [
        [
          "1234567"
        ]
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the comment is deleted.",
      "examples": [
        false
      ]
    },
    "posted_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Posted At",
      "description": "Date and time when the comment was posted, or `null` if unknown.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "reactions": {
      "anyOf": [
        {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Reactions",
      "description": "Reaction emoji mapped to user IDs that reacted.",
      "examples": [
        {
          "👍": [
            "1234567"
          ]
        }
      ]
    },
    "item_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Item Id",
      "description": "ID of the task the comment is on.",
      "examples": [
        "6X7gfV9G7rWm5hW8"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "posted_uid",
    "file_attachment",
    "uids_to_notify",
    "is_deleted",
    "posted_at",
    "reactions",
    "item_id"
  ],
  "title": "ItemNoteSyncView",
  "description": "A representation of an Item Note View, extending the base class NoteSyncView,\nto be returned to clients in a sync (or sync-like) response."
}
```

### ItemSyncView

```json
{
  "properties": {
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "String ID of the user who owns the task.",
      "examples": [
        "1234567"
      ]
    },
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the task.",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "String ID of the project that contains the task.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "section_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Section Id",
      "description": "String ID of the section that contains the task, or `null` if the task is not in a section.",
      "examples": [
        "6fFPHV272WWh3gpW"
      ]
    },
    "parent_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Parent Id",
      "description": "String ID of the parent task, or `null` if this is a top-level task.",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "added_by_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Added By Uid",
      "description": "String ID of the user who created the task, or `null` if unknown.",
      "examples": [
        "1234567"
      ]
    },
    "assigned_by_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Assigned By Uid",
      "description": "String ID of the user who assigned the task, or `null` if unassigned.",
      "examples": [
        "1234567"
      ]
    },
    "responsible_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Responsible Uid",
      "description": "String ID of the user responsible for the task, or `null` if unassigned.",
      "examples": [
        "1234567"
      ]
    },
    "labels": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Labels",
      "description": "Names of labels attached to the task.",
      "examples": [
        [
          "priority"
        ]
      ]
    },
    "deadline": {
      "anyOf": [
        {
          "additionalProperties": {
            "anyOf": [
              {
                "type": "string"
              },
              {
                "type": "null"
              }
            ]
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Deadline",
      "description": "Deadline details for the task, or `null` when the task has no deadline.",
      "examples": [
        {
          "date": "2025-02-12",
          "lang": "en"
        }
      ]
    },
    "duration": {
      "anyOf": [
        {
          "additionalProperties": {
            "anyOf": [
              {
                "type": "integer"
              },
              {
                "type": "string"
              }
            ]
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Duration",
      "description": "Task duration details, or `null` when the task has no duration.",
      "examples": [
        {
          "amount": 30,
          "unit": "minute"
        }
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Whether the task is collapsed in the user's view.",
      "examples": [
        false
      ]
    },
    "checked": {
      "type": "boolean",
      "title": "Checked",
      "description": "Whether the task is completed.",
      "examples": [
        false
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the task is deleted.",
      "examples": [
        false
      ]
    },
    "added_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Added At",
      "description": "Date and time when the task was created, or `null` if unknown.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "completed_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Completed At",
      "description": "Date and time when the task was completed, or `null` if active.",
      "examples": [
        "2025-01-16T10:30:00Z"
      ]
    },
    "completed_by_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Completed By Uid",
      "description": "String ID of the user who completed the task, or `null` if active.",
      "examples": [
        "1234567"
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
      "description": "Date and time when the task was last updated, or `null` if unknown.",
      "examples": [
        "2025-01-17T10:30:00Z"
      ]
    },
    "due": {
      "anyOf": [
        {
          "additionalProperties": true,
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due",
      "description": "Due date details for the task, or `null` when the task has no due date.",
      "examples": [
        {
          "date": "2025-02-12",
          "is_recurring": false,
          "lang": "en",
          "string": "tomorrow"
        }
      ]
    },
    "priority": {
      "type": "integer",
      "title": "Priority",
      "description": "Task priority from 1 (normal) to 4 (urgent).",
      "examples": [
        1
      ]
    },
    "child_order": {
      "type": "integer",
      "title": "Child Order",
      "description": "Position of the task among sibling tasks.",
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
      "description": "Fractional-indexing order key: tasks sort by comparing keys lexicographically among siblings sharing the same project, section and parent task. May be `null` for tasks not yet migrated.",
      "examples": [
        "a1V"
      ]
    },
    "content": {
      "type": "string",
      "title": "Content",
      "description": "Task content.",
      "examples": [
        "Buy milk"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Task description.",
      "examples": [
        "Pick up organic milk"
      ]
    },
    "note_count": {
      "type": "integer",
      "title": "Note Count",
      "description": "**Deprecated**: only returns 0 and is marked for removal.",
      "examples": [
        0
      ]
    },
    "day_order": {
      "type": "integer",
      "title": "Day Order",
      "description": "Task order for day-based views.",
      "examples": [
        1
      ]
    },
    "completed_count": {
      "type": "integer",
      "title": "Completed Count",
      "description": "Number of times the task has been marked as completed. Increments on every completion, including recurring occurrences and re-completions of a previously reopened task. Decremented when a recurring completion is undone.",
      "examples": [
        3
      ]
    },
    "postponed_count": {
      "type": "integer",
      "title": "Postponed Count",
      "description": "Number of times the task's due date has been rescheduled by the user. Increments on any due-date change between two set dates (forward or backward); recurring completions and undos are not counted.",
      "examples": [
        1
      ]
    }
  },
  "type": "object",
  "required": [
    "user_id",
    "id",
    "project_id",
    "section_id",
    "parent_id",
    "added_by_uid",
    "assigned_by_uid",
    "responsible_uid",
    "labels",
    "deadline",
    "duration",
    "is_collapsed",
    "checked",
    "is_deleted",
    "added_at",
    "completed_at",
    "completed_by_uid",
    "updated_at",
    "due",
    "priority",
    "child_order",
    "order_key",
    "content",
    "description",
    "note_count",
    "day_order",
    "completed_count",
    "postponed_count"
  ],
  "title": "ItemSyncView",
  "description": "A class with fields representing an ItemView which will be returned to\nclients in a sync (or sync-like) response."
}
```

### NotificationSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the reminder.",
      "examples": [
        "6XGgmFQrx44wfGHr"
      ]
    },
    "item_id": {
      "type": "string",
      "title": "Item Id",
      "description": "String ID of the task that owns the reminder.",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "notify_uid": {
      "type": "string",
      "title": "Notify Uid",
      "description": "String ID of the user who receives the reminder.",
      "examples": [
        "1234567"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the reminder is deleted.",
      "examples": [
        false
      ]
    },
    "is_urgent": {
      "type": "boolean",
      "title": "Is Urgent",
      "description": "Whether the reminder should be delivered urgently.",
      "examples": [
        false
      ]
    },
    "type": {
      "type": "string",
      "enum": [
        "absolute",
        "relative"
      ],
      "title": "Type",
      "description": "Reminder type.",
      "examples": [
        "relative"
      ]
    },
    "minute_offset": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Minute Offset",
      "description": "Minutes before the task due time for relative reminders.",
      "examples": [
        30
      ]
    },
    "due": {
      "anyOf": [
        {
          "additionalProperties": true,
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due",
      "description": "Datetime when the reminder fires, or `null` if unset.",
      "examples": [
        {
          "date": "2025-02-12T09:00:00",
          "lang": "en",
          "timezone": "Europe/Madrid"
        }
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "item_id",
    "notify_uid",
    "is_deleted",
    "is_urgent",
    "type",
    "minute_offset",
    "due"
  ],
  "title": "NotificationSyncView",
  "description": "A class with fields representing a NotificationView which will be returned\nto clients in a sync (or sync-like) response."
}
```

### PaginatedList_AnyProjectSyncViewResponse_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/AnyProjectSyncViewResponse"
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

### PaginatedList_Collaborator_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/Collaborator"
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

### PauseActivityEffect

```json
{
  "properties": {
    "scope": {
      "$ref": "#/components/schemas/EffectScope",
      "description": "May be omitted, which means `all`."
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "PauseActivityEffect"
}
```

### PauseNotificationsEffect

```json
{
  "properties": {
    "scope": {
      "$ref": "#/components/schemas/EffectScope",
      "description": "May be omitted, which means `all`."
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "PauseNotificationsEffect"
}
```

### PersonalProjectSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the project.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "can_assign_tasks": {
      "type": "boolean",
      "title": "Can Assign Tasks",
      "description": "Whether the current user can assign tasks in the project.",
      "examples": [
        true
      ]
    },
    "can_comment": {
      "type": "boolean",
      "title": "Can Comment",
      "description": "Whether the current user can comment in the project.",
      "examples": [
        true
      ]
    },
    "child_order": {
      "type": "integer",
      "title": "Child Order",
      "description": "Position of the project among sibling projects.",
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
      "description": "Fractional-indexing order key among sibling projects, or `null` until assigned (backfill in progress). Always `null` for workspace projects, which are ordered via folders instead.",
      "examples": [
        "a0"
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Whether the project is collapsed in the user's view.",
      "examples": [
        false
      ]
    },
    "color": {
      "type": "string",
      "title": "Color",
      "description": "Project color name.",
      "examples": [
        "charcoal"
      ]
    },
    "icon": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Icon",
      "description": "Project icon name, or `null` for the default icon. More icons will be added over time, so clients should fall back to the default icon for names they don't recognize.",
      "examples": [
        "lightbulb"
      ]
    },
    "creator_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Creator Uid",
      "description": "String ID of the user who created the project.",
      "examples": [
        "1234567"
      ]
    },
    "created_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Created At",
      "description": "Date and time when the project was created, or `null` if unknown.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "is_archived": {
      "type": "boolean",
      "title": "Is Archived",
      "description": "Whether the project is archived.",
      "examples": [
        false
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the project is deleted.",
      "examples": [
        false
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the project is marked as a favorite.",
      "examples": [
        false
      ]
    },
    "is_frozen": {
      "type": "boolean",
      "title": "Is Frozen",
      "description": "Whether the project is frozen and cannot be modified.",
      "examples": [
        false
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Project name.",
      "examples": [
        "Inbox"
      ]
    },
    "is_shared": {
      "type": "boolean",
      "title": "Is Shared",
      "description": "Whether the project is shared with other users.",
      "examples": [
        false
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
      "description": "Date and time when the project was last updated, or `null` if unknown.",
      "examples": [
        "2025-01-16T10:30:00Z"
      ]
    },
    "view_style": {
      "type": "string",
      "title": "View Style",
      "description": "Project view style.",
      "examples": [
        "list"
      ]
    },
    "default_order": {
      "type": "integer",
      "title": "Default Order",
      "description": "Default order of the project in team views.",
      "examples": [
        1
      ]
    },
    "default_order_key": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Default Order Key",
      "description": "Fractional-indexing key for the workspace's shared default-ordering scope (folders and workspace projects together). `null` for personal projects, and for workspace projects not yet assigned one (backfill pending).",
      "examples": [
        "a1V"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Project description.",
      "examples": [
        "Tasks for the launch plan"
      ]
    },
    "public_key": {
      "type": "string",
      "title": "Public Key",
      "description": "Public access key for the project, or an empty string when public access is disabled.",
      "examples": [
        "550e8400-e29b-41d4-a716-446655440000"
      ]
    },
    "access": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectAccessView"
        },
        {
          "type": "null"
        }
      ],
      "description": "Project access configuration.",
      "examples": [
        null
      ]
    },
    "role": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Role",
      "description": "Current user's role in the project.",
      "examples": [
        "admin"
      ]
    },
    "parent_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Parent Id",
      "description": "String ID of the parent project, or `null` for root projects.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "inbox_project": {
      "type": "boolean",
      "title": "Inbox Project",
      "description": "Whether this project is the user's Inbox project.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "can_assign_tasks",
    "can_comment",
    "child_order",
    "order_key",
    "is_collapsed",
    "color",
    "creator_uid",
    "created_at",
    "is_archived",
    "is_deleted",
    "is_favorite",
    "is_frozen",
    "name",
    "is_shared",
    "updated_at",
    "view_style",
    "default_order",
    "default_order_key",
    "description",
    "public_key",
    "access",
    "role",
    "parent_id",
    "inbox_project"
  ],
  "title": "PersonalProjectSyncView",
  "description": "A representation of a Personal Project Sync View, extending the base ProjectSync\nView, to be returned to clients in a sync (or sync-like) response."
}
```

### ProjectAccessView

```json
{
  "properties": {
    "visibility": {
      "$ref": "#/components/schemas/ProjectVisibility",
      "description": "Project visibility level.",
      "examples": [
        "public"
      ]
    },
    "configuration": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/RestrictedProjectConfiguration"
        },
        {
          "$ref": "#/components/schemas/TeamProjectConfiguration"
        },
        {
          "$ref": "#/components/schemas/PublicProjectConfiguration"
        }
      ],
      "title": "Configuration",
      "description": "Configuration for the selected visibility level.",
      "examples": [
        {}
      ]
    }
  },
  "type": "object",
  "required": [
    "visibility",
    "configuration"
  ],
  "title": "ProjectAccessView",
  "description": "Contains the details on who can view a project."
}
```

### ProjectImportCreateResponse

```json
{
  "properties": {
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "ID of the project created from the template.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Always `ok` on success.",
      "examples": [
        "ok"
      ]
    }
  },
  "type": "object",
  "required": [
    "project_id",
    "status"
  ],
  "title": "ProjectImportCreateResponse"
}
```

### ProjectImportCreateResponseWithObjects

```json
{
  "properties": {
    "project_view_options_defaults": {
      "items": {
        "$ref": "#/components/schemas/ImportedProjectViewOptionsDefault"
      },
      "type": "array",
      "title": "Project View Options Defaults",
      "description": "View options defaults applied to the imported project."
    },
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Always `ok` on success.",
      "examples": [
        "ok"
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "ID of the project created from the template.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "template_type": {
      "type": "string",
      "const": "project",
      "title": "Template Type",
      "description": "Type of template that was imported.",
      "examples": [
        "project"
      ]
    },
    "projects": {
      "items": {
        "anyOf": [
          {
            "$ref": "#/components/schemas/PersonalProjectSyncView"
          },
          {
            "$ref": "#/components/schemas/WorkspaceProjectSyncView"
          }
        ]
      },
      "type": "array",
      "title": "Projects",
      "description": "Projects created by the import.",
      "examples": [
        [
          {
            "access": {
              "configuration": {},
              "visibility": "restricted"
            },
            "can_assign_tasks": false,
            "can_comment": true,
            "child_order": 1,
            "color": "lime_green",
            "created_at": "2025-01-15T10:30:00Z",
            "creator_uid": "1234567",
            "default_order": 0,
            "description": "",
            "id": "6XGgm6PHrGgMpCFX",
            "is_archived": false,
            "is_collapsed": false,
            "is_deleted": false,
            "is_favorite": false,
            "is_frozen": false,
            "is_shared": false,
            "name": "Shopping List",
            "order_key": "a1V",
            "public_access": false,
            "public_key": "",
            "role": "CREATOR",
            "updated_at": "2025-01-16T10:30:00Z",
            "view_style": "list"
          }
        ]
      ]
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections created by the import.",
      "examples": [
        [
          {
            "added_at": "2025-01-15T10:30:00Z",
            "description": "Tasks for the launch plan",
            "id": "6fFPHV272WWh3gpW",
            "is_archived": false,
            "is_collapsed": false,
            "is_deleted": false,
            "name": "Introductions",
            "order_key": "a1V",
            "project_id": "6XGgm6PHrGgMpCFX",
            "section_order": 1,
            "updated_at": "2025-01-16T10:30:00Z",
            "user_id": "1234567"
          }
        ]
      ]
    },
    "tasks": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Tasks",
      "description": "Tasks created by the import.",
      "examples": [
        [
          {
            "added_at": "2025-01-15T10:30:00Z",
            "added_by_uid": "1234567",
            "checked": false,
            "child_order": 1,
            "completed_count": 3,
            "content": "Welcome the new hire",
            "day_order": -1,
            "description": "",
            "id": "6XGgmFVcrG5RRjVr",
            "is_collapsed": false,
            "is_deleted": false,
            "labels": [],
            "note_count": 0,
            "order_key": "a1V",
            "postponed_count": 1,
            "priority": 1,
            "project_id": "6XGgm6PHrGgMpCFX",
            "section_id": "6fFPHV272WWh3gpW",
            "updated_at": "2025-01-16T10:30:00Z",
            "user_id": "1234567"
          }
        ]
      ]
    },
    "comments": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Comments",
      "description": "Task comments created by the import.",
      "examples": [
        [
          {
            "content": "Great idea!",
            "id": "6Xw8x1xCFFw6Q5p9",
            "is_deleted": false,
            "item_id": "6XGgmFVcrG5RRjVr",
            "posted_at": "2025-01-15T10:30:00Z",
            "posted_uid": "1234567"
          }
        ]
      ]
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments created by the import.",
      "examples": [
        [
          {
            "content": "Project kickoff notes",
            "id": "6Xw8x1xCFFw6Q5q0",
            "is_deleted": false,
            "posted_at": "2025-01-15T10:30:00Z",
            "posted_uid": "1234567",
            "project_id": "6XGgm6PHrGgMpCFX"
          }
        ]
      ]
    },
    "reminders": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
      },
      "type": "array",
      "title": "Reminders",
      "description": "Reminders created by the import.",
      "examples": [
        [
          {
            "due": {
              "date": "2025-02-12T09:00:00",
              "lang": "en",
              "timezone": "Europe/Madrid"
            },
            "id": "6XGgmFQrx44wfGHr",
            "is_deleted": false,
            "is_urgent": false,
            "item_id": "6XGgmFVcrG5RRjVr",
            "minute_offset": 30,
            "notify_uid": "1234567",
            "type": "relative"
          }
        ]
      ]
    },
    "discarded_fields": {
      "items": {
        "$ref": "#/components/schemas/DiscardedField"
      },
      "type": "array",
      "title": "Discarded Fields",
      "description": "Task properties the importer dropped from one or more imported tasks because the plan in force does not include them. Empty when nothing was dropped. `reason` says which plan has to change: `workspace_plan_limit` for the workspace the project belongs to, `personal_plan_limit` for the importing user's own subscription.",
      "examples": [
        [
          {
            "field": "duration",
            "reason": "workspace_plan_limit"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "project_view_options_defaults",
    "status",
    "project_id",
    "template_type",
    "projects",
    "sections",
    "tasks",
    "comments",
    "project_notes",
    "reminders",
    "discarded_fields"
  ],
  "title": "ProjectImportCreateResponseWithObjects"
}
```

### ProjectImportResponse

```json
{
  "properties": {
    "project_view_options_defaults": {
      "items": {
        "$ref": "#/components/schemas/ImportedProjectViewOptionsDefault"
      },
      "type": "array",
      "title": "Project View Options Defaults",
      "description": "View options defaults applied to the imported project."
    },
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Always `ok` on success.",
      "examples": [
        "ok"
      ]
    },
    "template_type": {
      "type": "string",
      "const": "project",
      "title": "Template Type",
      "description": "Type of template that was imported.",
      "examples": [
        "project"
      ]
    },
    "projects": {
      "items": {
        "anyOf": [
          {
            "$ref": "#/components/schemas/PersonalProjectSyncView"
          },
          {
            "$ref": "#/components/schemas/WorkspaceProjectSyncView"
          }
        ]
      },
      "type": "array",
      "title": "Projects",
      "description": "Projects created or updated by the import.",
      "examples": [
        [
          {
            "access": {
              "configuration": {},
              "visibility": "restricted"
            },
            "can_assign_tasks": false,
            "can_comment": true,
            "child_order": 1,
            "color": "lime_green",
            "created_at": "2025-01-15T10:30:00Z",
            "creator_uid": "1234567",
            "default_order": 0,
            "description": "",
            "id": "6XGgm6PHrGgMpCFX",
            "is_archived": false,
            "is_collapsed": false,
            "is_deleted": false,
            "is_favorite": false,
            "is_frozen": false,
            "is_shared": false,
            "name": "Shopping List",
            "order_key": "a1V",
            "public_access": false,
            "public_key": "",
            "role": "CREATOR",
            "updated_at": "2025-01-16T10:30:00Z",
            "view_style": "list"
          }
        ]
      ]
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections created by the import.",
      "examples": [
        [
          {
            "added_at": "2025-01-15T10:30:00Z",
            "description": "Tasks for the launch plan",
            "id": "6fFPHV272WWh3gpW",
            "is_archived": false,
            "is_collapsed": false,
            "is_deleted": false,
            "name": "Introductions",
            "order_key": "a1V",
            "project_id": "6XGgm6PHrGgMpCFX",
            "section_order": 1,
            "updated_at": "2025-01-16T10:30:00Z",
            "user_id": "1234567"
          }
        ]
      ]
    },
    "tasks": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Tasks",
      "description": "Tasks created by the import.",
      "examples": [
        [
          {
            "added_at": "2025-01-15T10:30:00Z",
            "added_by_uid": "1234567",
            "checked": false,
            "child_order": 1,
            "completed_count": 3,
            "content": "Welcome the new hire",
            "day_order": -1,
            "description": "",
            "id": "6XGgmFVcrG5RRjVr",
            "is_collapsed": false,
            "is_deleted": false,
            "labels": [],
            "note_count": 0,
            "order_key": "a1V",
            "postponed_count": 1,
            "priority": 1,
            "project_id": "6XGgm6PHrGgMpCFX",
            "section_id": "6fFPHV272WWh3gpW",
            "updated_at": "2025-01-16T10:30:00Z",
            "user_id": "1234567"
          }
        ]
      ]
    },
    "comments": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Comments",
      "description": "Task comments created by the import.",
      "examples": [
        [
          {
            "content": "Great idea!",
            "id": "6Xw8x1xCFFw6Q5p9",
            "is_deleted": false,
            "item_id": "6XGgmFVcrG5RRjVr",
            "posted_at": "2025-01-15T10:30:00Z",
            "posted_uid": "1234567"
          }
        ]
      ]
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments created by the import.",
      "examples": [
        [
          {
            "content": "Project kickoff notes",
            "id": "6Xw8x1xCFFw6Q5q0",
            "is_deleted": false,
            "posted_at": "2025-01-15T10:30:00Z",
            "posted_uid": "1234567",
            "project_id": "6XGgm6PHrGgMpCFX"
          }
        ]
      ]
    },
    "reminders": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
      },
      "type": "array",
      "title": "Reminders",
      "description": "Reminders created by the import.",
      "examples": [
        [
          {
            "due": {
              "date": "2025-02-12T09:00:00",
              "lang": "en",
              "timezone": "Europe/Madrid"
            },
            "id": "6XGgmFQrx44wfGHr",
            "is_deleted": false,
            "is_urgent": false,
            "item_id": "6XGgmFVcrG5RRjVr",
            "minute_offset": 30,
            "notify_uid": "1234567",
            "type": "relative"
          }
        ]
      ]
    },
    "discarded_fields": {
      "items": {
        "$ref": "#/components/schemas/DiscardedField"
      },
      "type": "array",
      "title": "Discarded Fields",
      "description": "Task properties the importer dropped from one or more imported tasks because the plan in force does not include them. Empty when nothing was dropped. `reason` says which plan has to change: `workspace_plan_limit` for the workspace the project belongs to, `personal_plan_limit` for the importing user's own subscription.",
      "examples": [
        [
          {
            "field": "duration",
            "reason": "workspace_plan_limit"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "project_view_options_defaults",
    "status",
    "template_type",
    "projects",
    "sections",
    "tasks",
    "comments",
    "project_notes",
    "reminders",
    "discarded_fields"
  ],
  "title": "ProjectImportResponse"
}
```

### ProjectNoteSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the comment.",
      "examples": [
        "6XGgmFQrx44wfGHr"
      ]
    },
    "posted_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted Uid",
      "description": "String ID of the user who posted the comment.",
      "examples": [
        "1234567"
      ]
    },
    "content": {
      "type": "string",
      "title": "Content",
      "description": "Comment content.",
      "default": "",
      "examples": [
        "Looks good to me."
      ]
    },
    "file_attachment": {
      "anyOf": [
        {
          "additionalProperties": {
            "anyOf": [
              {
                "type": "string"
              },
              {
                "type": "integer"
              },
              {
                "items": {},
                "type": "array"
              },
              {
                "additionalProperties": true,
                "type": "object"
              },
              {
                "type": "null"
              }
            ]
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Attachment",
      "description": "File attachment metadata, or `null` when the comment has no attachment.",
      "examples": [
        null
      ]
    },
    "uids_to_notify": {
      "anyOf": [
        {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        {
          "type": "null"
        }
      ],
      "title": "Uids To Notify",
      "description": "String IDs of users notified by the comment.",
      "examples": [
        [
          "1234567"
        ]
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the comment is deleted.",
      "examples": [
        false
      ]
    },
    "posted_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Posted At",
      "description": "Date and time when the comment was posted, or `null` if unknown.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "reactions": {
      "anyOf": [
        {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array"
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Reactions",
      "description": "Reaction emoji mapped to user IDs that reacted.",
      "examples": [
        {
          "👍": [
            "1234567"
          ]
        }
      ]
    },
    "project_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Project Id",
      "description": "String ID of the project the comment belongs to.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "posted_uid",
    "file_attachment",
    "uids_to_notify",
    "is_deleted",
    "posted_at",
    "reactions",
    "project_id"
  ],
  "title": "ProjectNoteSyncView",
  "description": "A representation of a Project Note View, extending the base class NoteSyncView,\nto be returned to clients in a sync (or sync-like) response."
}
```

### ProjectStatus

```json
{
  "type": "string",
  "enum": [
    "PLANNED",
    "IN_PROGRESS",
    "PAUSED",
    "COMPLETED",
    "CANCELED"
  ],
  "title": "ProjectStatus",
  "description": "Project status.\n\nAt the moment, this is for workspace projects only.",
  "x-enumDescriptions": {
    "CANCELED": "Canceled",
    "COMPLETED": "Completed",
    "IN_PROGRESS": "In Progress",
    "PAUSED": "Paused",
    "PLANNED": "Planned"
  }
}
```

### ProjectViewStyleV1

```json
{
  "type": "string",
  "enum": [
    "list",
    "board",
    "calendar",
    "table"
  ]
}
```

### ProjectVisibility

```json
{
  "type": "string",
  "enum": [
    "restricted",
    "team",
    "public"
  ],
  "title": "ProjectVisibility",
  "description": "Indicates who can see a project",
  "x-enumDescriptions": {
    "public": "Public",
    "restricted": "Restricted",
    "team": "Team"
  }
}
```

### PublicProjectConfiguration

```json
{
  "properties": {
    "hide_collaborator_details": {
      "type": "boolean",
      "title": "Hide Collaborator Details",
      "description": "Whether collaborator details are hidden on the public project.",
      "examples": [
        true
      ]
    },
    "disable_duplication": {
      "type": "boolean",
      "title": "Disable Duplication",
      "description": "Whether public project duplication is disabled.",
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "required": [
    "hide_collaborator_details",
    "disable_duplication"
  ],
  "title": "PublicProjectConfiguration",
  "description": "Configuration specific to public projects."
}
```

### RestrictedProjectConfiguration

```json
{
  "properties": {},
  "type": "object",
  "title": "RestrictedProjectConfiguration",
  "description": "Configuration specific to restricted projects."
}
```

### RoleToActionMappingView

```json
{
  "properties": {
    "project_collaborator_actions": {
      "items": {
        "$ref": "#/components/schemas/RoleView"
      },
      "type": "array",
      "title": "Project Collaborator Actions",
      "description": "Project collaborator roles and their allowed actions.",
      "examples": [
        [
          {
            "actions": [
              {
                "name": "UPDATE_ITEM"
              }
            ],
            "name": "MEMBER"
          }
        ]
      ]
    },
    "workspace_collaborator_actions": {
      "items": {
        "$ref": "#/components/schemas/RoleView"
      },
      "type": "array",
      "title": "Workspace Collaborator Actions",
      "description": "Workspace collaborator roles and their allowed actions.",
      "examples": [
        [
          {
            "actions": [
              {
                "name": "UPDATE_ITEM"
              }
            ],
            "name": "MEMBER"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "project_collaborator_actions",
    "workspace_collaborator_actions"
  ],
  "title": "RoleToActionMappingView"
}
```

### RoleView

```json
{
  "properties": {
    "name": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CollaboratorRole"
        },
        {
          "$ref": "#/components/schemas/WorkspaceRole"
        }
      ],
      "title": "Name",
      "description": "Role that grants the listed actions.",
      "examples": [
        "MEMBER"
      ]
    },
    "actions": {
      "items": {
        "$ref": "#/components/schemas/ActionView"
      },
      "type": "array",
      "title": "Actions",
      "description": "Actions granted to this role.",
      "examples": [
        [
          {
            "name": "UPDATE_ITEM"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "name",
    "actions"
  ],
  "title": "RoleView"
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

### TeamProjectConfiguration

```json
{
  "properties": {},
  "type": "object",
  "title": "TeamProjectConfiguration",
  "description": "Configuration specific to team projects."
}
```

### UserStatus

```json
{
  "properties": {
    "message": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 80,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Message",
      "description": "The status text. Surrounding whitespace is trimmed. `null` when the status has only an emoji.",
      "examples": [
        "On holiday"
      ]
    },
    "emoji": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Emoji",
      "description": "An optional single emoji.",
      "examples": [
        "⛱️"
      ]
    },
    "active_until": {
      "anyOf": [
        {
          "type": "string",
          "format": "date-time"
        },
        {
          "type": "null"
        }
      ],
      "title": "Active Until",
      "description": "When the status and its effects expire, in UTC. `null` or omitted means it stays until cleared.",
      "examples": [
        "2026-09-08T15:00:00Z"
      ]
    },
    "effects": {
      "$ref": "#/components/schemas/UserStatusEffects"
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "UserStatus",
  "description": "A complete stored status. Unknown stored fields survive partial updates."
}
```

### UserStatusEffects

```json
{
  "properties": {
    "pause_notifications": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PauseNotificationsEffect"
        },
        {
          "type": "null"
        }
      ],
      "description": "Pauses the user's notifications, in products that support it, while the status is active."
    },
    "pause_activity": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PauseActivityEffect"
        },
        {
          "type": "null"
        }
      ],
      "description": "Pauses the user's productivity metrics (Karma and streaks) in Todoist while the status is active, as Vacation Mode does."
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "UserStatusEffects",
  "description": "The optional behaviors a status enables. Empty for an informational status.\nEach effect is independent: a status may carry either, both, or neither."
}
```

### ViewMode

```json
{
  "type": "string",
  "enum": [
    "LIST",
    "BOARD",
    "CALENDAR",
    "TABLE"
  ],
  "title": "ViewMode",
  "x-enumDescriptions": {
    "BOARD": "Board",
    "CALENDAR": "Calendar",
    "LIST": "List",
    "TABLE": "Table"
  }
}
```

### WorkspaceProjectSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the project.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "can_assign_tasks": {
      "type": "boolean",
      "title": "Can Assign Tasks",
      "description": "Whether the current user can assign tasks in the project.",
      "examples": [
        true
      ]
    },
    "can_comment": {
      "type": "boolean",
      "title": "Can Comment",
      "description": "Whether the current user can comment in the project.",
      "examples": [
        true
      ]
    },
    "child_order": {
      "type": "integer",
      "title": "Child Order",
      "description": "Position of the project among sibling projects.",
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
      "description": "Fractional-indexing order key among sibling projects, or `null` until assigned (backfill in progress). Always `null` for workspace projects, which are ordered via folders instead.",
      "examples": [
        "a0"
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Whether the project is collapsed in the user's view.",
      "examples": [
        false
      ]
    },
    "color": {
      "type": "string",
      "title": "Color",
      "description": "Project color name.",
      "examples": [
        "charcoal"
      ]
    },
    "icon": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Icon",
      "description": "Project icon name, or `null` for the default icon. More icons will be added over time, so clients should fall back to the default icon for names they don't recognize.",
      "examples": [
        "lightbulb"
      ]
    },
    "creator_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Creator Uid",
      "description": "String ID of the user who created the project.",
      "examples": [
        "1234567"
      ]
    },
    "created_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Created At",
      "description": "Date and time when the project was created, or `null` if unknown.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "is_archived": {
      "type": "boolean",
      "title": "Is Archived",
      "description": "Whether the project is archived.",
      "examples": [
        false
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the project is deleted.",
      "examples": [
        false
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the project is marked as a favorite.",
      "examples": [
        false
      ]
    },
    "is_frozen": {
      "type": "boolean",
      "title": "Is Frozen",
      "description": "Whether the project is frozen and cannot be modified.",
      "examples": [
        false
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Project name.",
      "examples": [
        "Inbox"
      ]
    },
    "is_shared": {
      "type": "boolean",
      "title": "Is Shared",
      "description": "Whether the project is shared with other users.",
      "examples": [
        false
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
      "description": "Date and time when the project was last updated, or `null` if unknown.",
      "examples": [
        "2025-01-16T10:30:00Z"
      ]
    },
    "view_style": {
      "type": "string",
      "title": "View Style",
      "description": "Project view style.",
      "examples": [
        "list"
      ]
    },
    "default_order": {
      "type": "integer",
      "title": "Default Order",
      "description": "Default order of the project in team views.",
      "examples": [
        1
      ]
    },
    "default_order_key": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Default Order Key",
      "description": "Fractional-indexing key for the workspace's shared default-ordering scope (folders and workspace projects together). `null` for personal projects, and for workspace projects not yet assigned one (backfill pending).",
      "examples": [
        "a1V"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Project description.",
      "examples": [
        "Tasks for the launch plan"
      ]
    },
    "public_key": {
      "type": "string",
      "title": "Public Key",
      "description": "Public access key for the project, or an empty string when public access is disabled.",
      "examples": [
        "550e8400-e29b-41d4-a716-446655440000"
      ]
    },
    "access": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectAccessView"
        },
        {
          "type": "null"
        }
      ],
      "description": "Project access configuration.",
      "examples": [
        null
      ]
    },
    "role": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Role",
      "description": "Current user's role in the project.",
      "examples": [
        "admin"
      ]
    },
    "collaborator_role_default": {
      "type": "string",
      "title": "Collaborator Role Default",
      "description": "Default collaborator role for the workspace project.",
      "examples": [
        "member"
      ]
    },
    "folder_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Folder Id",
      "description": "String ID of the folder that contains the project, or `null`.",
      "examples": [
        "1234"
      ]
    },
    "is_invite_only": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Invite Only",
      "description": "Whether joining the workspace project requires an invitation.",
      "examples": [
        false
      ]
    },
    "is_link_sharing_enabled": {
      "type": "boolean",
      "title": "Is Link Sharing Enabled",
      "description": "Whether invitation links are enabled for the project.",
      "examples": [
        false
      ]
    },
    "status": {
      "$ref": "#/components/schemas/ProjectStatus",
      "description": "Project status.",
      "examples": [
        "in_progress"
      ]
    },
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "String ID of the workspace that contains the project.",
      "examples": [
        "123456"
      ]
    },
    "is_pending_default_collaborator_invites": {
      "type": "boolean",
      "title": "Is Pending Default Collaborator Invites",
      "description": "Whether default collaborator invitations are still being created in the background.",
      "examples": [
        false
      ]
    },
    "is_project_insights_enabled": {
      "type": "boolean",
      "title": "Is Project Insights Enabled",
      "description": "Whether project insights are enabled for the project.",
      "examples": [
        true
      ]
    },
    "parent_id": {
      "type": "null",
      "title": "Parent Id",
      "description": "Always null for workspace projects."
    }
  },
  "type": "object",
  "required": [
    "id",
    "can_assign_tasks",
    "can_comment",
    "child_order",
    "order_key",
    "is_collapsed",
    "color",
    "creator_uid",
    "created_at",
    "is_archived",
    "is_deleted",
    "is_favorite",
    "is_frozen",
    "name",
    "is_shared",
    "updated_at",
    "view_style",
    "default_order",
    "default_order_key",
    "description",
    "public_key",
    "access",
    "role",
    "collaborator_role_default",
    "folder_id",
    "is_invite_only",
    "is_link_sharing_enabled",
    "status",
    "workspace_id",
    "is_pending_default_collaborator_invites",
    "is_project_insights_enabled"
  ],
  "title": "WorkspaceProjectSyncView",
  "description": "A representation of a Workspace Project Sync View, extending the base ProjectSync\nView, to be returned to clients in a sync (or sync-like) response."
}
```

### WorkspaceRole

```json
{
  "type": "string",
  "enum": [
    "ADMIN",
    "MEMBER",
    "GUEST"
  ],
  "title": "WorkspaceRole",
  "description": "Role of the user inside the workspace",
  "x-enumDescriptions": {
    "ADMIN": "Admin",
    "GUEST": "Guest",
    "MEMBER": "Member"
  }
}
```

### _GetDataV2Response

```json
{
  "properties": {
    "project": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PersonalProjectSyncView"
        },
        {
          "$ref": "#/components/schemas/WorkspaceProjectSyncView"
        },
        {
          "type": "null"
        }
      ],
      "title": "Project",
      "description": "Project details returned after joining."
    },
    "tasks": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Tasks",
      "description": "Tasks in the joined project."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the joined project."
    },
    "comments": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Comments",
      "description": "Comments in the joined project."
    },
    "collaborators": {
      "items": {
        "$ref": "#/components/schemas/ExposedCollaborator"
      },
      "type": "array",
      "title": "Collaborators",
      "description": "Collaborators with access to the joined project.",
      "examples": [
        [
          {
            "email": "ada@example.com",
            "full_name": "Ada Lovelace",
            "id": "123456",
            "is_agent": true,
            "is_deleted": false,
            "timezone": "Europe/London"
          }
        ]
      ]
    },
    "collaborator_states": {
      "items": {
        "additionalProperties": true,
        "type": "object"
      },
      "type": "array",
      "title": "Collaborator States",
      "description": "Collaborator state records for the joined project.",
      "examples": [
        [
          {
            "is_deleted": false,
            "project_id": "6XGgff2vcGGQCQvj",
            "state": "active",
            "user_id": "2671355"
          }
        ]
      ]
    },
    "folder": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/FolderView"
        },
        {
          "type": "null"
        }
      ],
      "description": "Folder containing the project, or `null` when it is not in a folder.",
      "examples": [
        null
      ]
    },
    "subprojects": {
      "items": {
        "anyOf": [
          {
            "$ref": "#/components/schemas/PersonalProjectSyncView"
          },
          {
            "$ref": "#/components/schemas/WorkspaceProjectSyncView"
          }
        ]
      },
      "type": "array",
      "title": "Subprojects",
      "description": "Subprojects of the joined project."
    }
  },
  "type": "object",
  "required": [
    "project",
    "tasks",
    "sections",
    "comments",
    "collaborators",
    "collaborator_states",
    "folder",
    "subprojects"
  ],
  "title": "_GetDataV2Response"
}
```
