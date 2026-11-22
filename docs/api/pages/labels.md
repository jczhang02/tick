# Labels

## Endpoints

### Shared Labels Remove

`POST /api/v1/labels/shared/remove`

Remove the given shared label from all active tasks

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_0680de45"
        }
      }
    },
    "required": true
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

### Shared Labels Rename

`POST /api/v1/labels/shared/rename`

Rename the given shared label from all active tasks

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_6ec83bd4"
        }
      }
    },
    "required": true
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

### Search Labels

`GET /api/v1/labels/search`

Search user labels by name.

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
        "description": "Search query to match label names. Matching is case-insensitive. Queries are matched literally unless `*` (wildcard) is included. Use `\\*` for literal asterisk and `\\\\` for literal backslash.",
        "examples": [
          "urgent",
          "priority-*",
          "*-review",
          "5\\*"
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
            "$ref": "#/components/schemas/PaginatedList_LabelRestView_"
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

### Shared Labels

`GET /api/v1/labels/shared`

Returns a set of unique strings containing [shared
labels](https://www.todoist.com/help/articles/introduction-to-labels-dSo2eE#h_01HE1NNWMV8MCXNVF10GT13ZEX)
from active tasks.

```json
{
  "parameters": [
    {
      "name": "omit_personal",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Whether to exclude the user's personal labels from the shared label names.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Omit Personal"
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
            "$ref": "#/components/schemas/PaginatedList_SharedLabelName_"
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

### Delete Label

`DELETE /api/v1/labels/{label_id}`

Deletes a personal label. All instances of the label will be removed from tasks

```json
{
  "parameters": [
    {
      "name": "label_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "String ID of the label",
        "examples": [
          "2147509004"
        ],
        "title": "Label Id"
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

### Get Label

`GET /api/v1/labels/{label_id}`

Return a personal label by ID.

Returns `NOT_FOUND` when the label does not exist or the ID is invalid.

```json
{
  "parameters": [
    {
      "name": "label_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "String ID of the label",
        "examples": [
          "2147509004"
        ],
        "title": "Label Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/LabelRestView"
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

### Update Label

`POST /api/v1/labels/{label_id}`

Update a personal label.

At least one mutable field must be provided. Passing `null` for an optional
field keeps the existing value unchanged.

```json
{
  "parameters": [
    {
      "name": "label_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "String ID of the label",
        "examples": [
          "2147509004"
        ],
        "title": "Label Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_9c41452d"
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
            "$ref": "#/components/schemas/LabelRestView"
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

### Create Label

`POST /api/v1/labels`

Create a personal label.

Premium limits apply to personal label creation. Free users who exceed the
plan limit receive a premium-only error.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_38bb43e2"
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
            "$ref": "#/components/schemas/LabelRestView"
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

### Get Labels

`GET /api/v1/labels`

Get all user labels.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for details on using cursor-based pagination.

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
            "$ref": "#/components/schemas/PaginatedList_LabelRestView_"
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

### Body_0680de45

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Shared label name to remove from matching tasks.",
      "examples": [
        "waiting"
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

### Body_38bb43e2

```json
{
  "properties": {
    "name": {
      "type": "string",
      "maxLength": 128,
      "minLength": 1,
      "title": "Name",
      "description": "Name of the new label.",
      "examples": [
        "Priority"
      ]
    },
    "order": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 32767.0,
          "minimum": -32768.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Order",
      "description": "Position of the new label in the label list",
      "examples": [
        12
      ]
    },
    "color": {
      "$ref": "#/components/schemas/APIColorID",
      "description": "Label color.",
      "default": "charcoal",
      "examples": [
        "berry_red"
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the label is marked as a favorite.",
      "default": false,
      "examples": [
        false
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

### Body_6ec83bd4

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Current shared label name to rename.",
      "examples": [
        "waiting"
      ]
    },
    "new_name": {
      "type": "string",
      "title": "New Name",
      "description": "New shared label name to apply to matching tasks.",
      "examples": [
        "follow-up"
      ]
    }
  },
  "type": "object",
  "required": [
    "name",
    "new_name"
  ],
  "title": "Body"
}
```

### Body_9c41452d

```json
{
  "properties": {
    "name": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 128,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Name",
      "description": "Updated label name. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Priority"
      ]
    },
    "order": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 32767.0,
          "minimum": -32768.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Order",
      "description": "Position of the label in the label list. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        12
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
      "description": "Label color. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "berry_red"
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
      "description": "Whether the label is marked as a favorite. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### LabelRestView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the label.",
      "examples": [
        "2147509004"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Label name.",
      "examples": [
        "Priority"
      ]
    },
    "color": {
      "type": "string",
      "title": "Color",
      "description": "Label color name.",
      "examples": [
        "berry_red"
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
      "description": "Position of the label in the label list, or `null` when no explicit order is set.",
      "examples": [
        12
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the label is marked as a favorite.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "color",
    "order",
    "is_favorite"
  ],
  "title": "LabelRestView"
}
```

### PaginatedList_LabelRestView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/LabelRestView"
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

### PaginatedList_SharedLabelName_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/SharedLabelName"
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

### SharedLabelName

```json
{
  "type": "string",
  "description": "Name of a shared label.",
  "examples": [
    "errands"
  ]
}
```
