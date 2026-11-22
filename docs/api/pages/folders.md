# Folders

## Endpoints

### Delete Folder

`DELETE /api/v1/folders/{folder_id}`

Delete a folder. Projects in the folder will be moved out of it.

```json
{
  "parameters": [
    {
      "name": "folder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the folder to delete.",
        "examples": [
          1234
        ],
        "title": "Folder Id"
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

### Get Folder

`GET /api/v1/folders/{folder_id}`

Return the folder for the given folder ID.

```json
{
  "parameters": [
    {
      "name": "folder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the folder to return.",
        "examples": [
          1234
        ],
        "title": "Folder Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/FolderSyncView"
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

### Update Folder

`POST /api/v1/folders/{folder_id}`

Update an existing folder.

```json
{
  "parameters": [
    {
      "name": "folder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the folder to update.",
        "examples": [
          1234
        ],
        "title": "Folder Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_09503d19"
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
            "$ref": "#/components/schemas/FolderSyncView"
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

### Create Folder

`POST /api/v1/folders`

Create a new folder in the given workspace.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_c04ca410"
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
            "$ref": "#/components/schemas/FolderSyncView"
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

### Get Folders

`GET /api/v1/folders`

Get all folders for a workspace.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the workspace to list folders for.",
        "examples": [
          123456
        ],
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
            "$ref": "#/components/schemas/PaginatedList_FolderSyncView_"
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

### Body_09503d19

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
      "description": "Updated folder name. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Design"
      ]
    },
    "default_order": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Default Order",
      "description": "Updated default position of the folder. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        1
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_c04ca410

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the new folder.",
      "examples": [
        "Design"
      ]
    },
    "workspace_id": {
      "type": "integer",
      "title": "Workspace Id",
      "description": "ID of the workspace to create the folder in.",
      "examples": [
        123456
      ]
    },
    "default_order": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Default Order",
      "description": "Default position of the folder in the workspace.",
      "examples": [
        1
      ]
    },
    "child_order": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Child Order",
      "description": "User-specific position of the folder.",
      "examples": [
        1
      ]
    }
  },
  "type": "object",
  "required": [
    "name",
    "workspace_id"
  ],
  "title": "Body"
}
```

### FolderSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the folder.",
      "examples": [
        "1234"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Folder name.",
      "examples": [
        "Design"
      ]
    },
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "String ID of the workspace that contains the folder.",
      "examples": [
        "123456"
      ]
    },
    "default_order": {
      "type": "integer",
      "title": "Default Order",
      "description": "Default position of the folder in the workspace.",
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
      "examples": [
        1
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the folder is deleted.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "workspace_id",
    "default_order",
    "default_order_key",
    "child_order",
    "is_deleted"
  ],
  "title": "FolderSyncView"
}
```

### PaginatedList_FolderSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/FolderSyncView"
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
