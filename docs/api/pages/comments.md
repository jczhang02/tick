# Comments

## Endpoints

### Remove Comment Reaction

`POST /api/v1/comments/{comment_id}/reactions/remove`

Removes the authenticated user's reaction from a task or project comment.

Removing a reaction the user has not added succeeds without changing the reactions.
Other users' reactions are not affected.

```json
{
  "parameters": [
    {
      "name": "comment_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the comment",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Comment Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_3b3999fa"
        }
      }
    }
  },
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

### Add Comment Reaction

`POST /api/v1/comments/{comment_id}/reactions`

Adds the authenticated user's reaction to a task or project comment and returns the updated comment.

Adding a reaction the user has already added leaves the reactions unchanged.
Returns an error if the comment's stored reactions would exceed 65,535 bytes.

```json
{
  "parameters": [
    {
      "name": "comment_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the comment",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Comment Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_579e1668"
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
            "$ref": "#/components/schemas/NoteSyncView"
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

### Delete Comment

`DELETE /api/v1/comments/{comment_id}`

Delete a comment by ID

```json
{
  "parameters": [
    {
      "name": "comment_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the comment",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Comment Id"
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

### Get Comment

`GET /api/v1/comments/{comment_id}`

Returns a single comment by ID

```json
{
  "parameters": [
    {
      "name": "comment_id",
      "in": "path",
      "required": true,
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "String ID of the comment",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Comment Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/NoteSyncView"
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

### Update Comment

`POST /api/v1/comments/{comment_id}`

Update a comment by ID and returns its content

```json
{
  "parameters": [
    {
      "name": "comment_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the comment",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Comment Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_61d93e0e"
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
            "anyOf": [
              {
                "$ref": "#/components/schemas/NoteSyncView"
              },
              {
                "type": "null"
              }
            ],
            "title": "Response Update Comment Api V1 Comments  Comment Id  Post"
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

### Create Comment

`POST /api/v1/comments`

Creates a new comment on a project or task and returns it.

Exactly one of `task_id` or `project_id` arguments is required. Providing
neither or both will return an error.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_28d2b1b0"
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
            "$ref": "#/components/schemas/NoteSyncView"
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

```curl

$ cat > /tmp/note.json
{
    "task_id": "6X6WMMqgq2PWxjCX",
    "content": "Need one bottle of milk",
    "attachment": {
        "resource_type": "file",
        "file_url": "https://s3.amazonaws.com/domorebetter/Todoist+Setup+Guide.pdf",
        "file_type": "application/pdf",
        "file_name": "File.pdf"
    }
}
^C

$ curl "https://api.todoist.com/api/v1/comments" \
    -X POST \
    --data @/tmp/note.json \
    -H "Content-Type: application/json" \
    -H "X-Request-Id: $(uuidgen)" \
    -H "Authorization: Bearer $token"
```

### Get Comments

`GET /api/v1/comments`

Get all comments for a given task or project.

Exactly one of `task_id` or `project_id` arguments is required. Providing
neither or both will return an error.

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
        "description": "String ID of the project",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "task_id",
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
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
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
            "$ref": "#/components/schemas/PaginatedList_NoteSyncView_"
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

### Body_28d2b1b0

```json
{
  "properties": {
    "content": {
      "type": "string",
      "maxLength": 15000,
      "minLength": 1,
      "title": "Content",
      "description": "Content of the comment",
      "examples": [
        "Need one bottle of milk"
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
      "description": "String ID of the project",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "task_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Task Id",
      "description": "String ID of the task",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "attachment": {
      "anyOf": [
        {
          "additionalProperties": true,
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Attachment",
      "description": "A [File attachment](https://developer.todoist.com/api/v1/sync/comments.md) object",
      "examples": [
        {
          "file_name": "File.pdf",
          "file_type": "application/pdf",
          "file_url": "https://s3.amazonaws.com/domorebetter/Todoist+Setup+Guide.pdf",
          "resource_type": "file"
        }
      ]
    },
    "uids_to_notify": {
      "anyOf": [
        {
          "items": {
            "type": "integer"
          },
          "type": "array"
        },
        {
          "type": "null"
        }
      ],
      "title": "Uids To Notify",
      "description": "Optional list of user IDs to notify about this comment.",
      "examples": [
        [
          12345678,
          23456789
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "content"
  ],
  "title": "Body"
}
```

### Body_3b3999fa

```json
{
  "properties": {
    "reaction": {
      "type": "string",
      "minLength": 1,
      "title": "Reaction",
      "description": "Reaction emoji.",
      "examples": [
        "👍"
      ]
    }
  },
  "type": "object",
  "required": [
    "reaction"
  ],
  "title": "Body"
}
```

### Body_579e1668

```json
{
  "properties": {
    "reaction": {
      "type": "string",
      "maxLength": 64,
      "minLength": 1,
      "title": "Reaction",
      "description": "Reaction emoji.",
      "examples": [
        "👍"
      ]
    }
  },
  "type": "object",
  "required": [
    "reaction"
  ],
  "title": "Body"
}
```

### Body_61d93e0e

```json
{
  "properties": {
    "content": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 15000,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Content",
      "description": "New content for the comment. If null or an empty string, no update is performed.",
      "examples": [
        "Need two bottles of milk"
      ]
    }
  },
  "type": "object",
  "required": [
    "content"
  ],
  "title": "Body"
}
```

### NoteSyncView

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
    "reactions"
  ],
  "title": "NoteSyncView",
  "description": "The base class for NoteSyncViews, to be extended for Items and Projects.\n\nThis class should generally not be instantiated directly, as it serves as a\ncommon structure for Items and Projects."
}
```

### PaginatedList_NoteSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/NoteSyncView"
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
