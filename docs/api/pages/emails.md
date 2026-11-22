# Emails

## Endpoints

### Email Disable

`DELETE /api/v1/emails`

Disable the current email to a Todoist object

```json
{
  "parameters": [
    {
      "name": "obj_type",
      "in": "query",
      "required": true,
      "schema": {
        "$ref": "#/components/schemas/EmailObjectType",
        "description": "Type of Todoist object whose email should be disabled.",
        "examples": [
          "project",
          "project_comments",
          "task"
        ]
      }
    },
    {
      "name": "obj_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the Todoist object.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Obj Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_StatusOkResponse"
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

### Email Get Or Create

`PUT /api/v1/emails`

Get or create an email to a Todoist object,
currently only projects and tasks are supported.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_0dc72dfd"
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
            "$ref": "#/components/schemas/_EmailResponse"
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

### Body_0dc72dfd

```json
{
  "properties": {
    "obj_type": {
      "$ref": "#/components/schemas/EmailObjectType",
      "title": "Obj Type",
      "description": "Type of Todoist object to create an email for.",
      "examples": [
        "project",
        "project_comments",
        "task"
      ]
    },
    "obj_id": {
      "type": "string",
      "title": "Obj Id",
      "description": "String ID of the Todoist object.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    }
  },
  "type": "object",
  "required": [
    "obj_type",
    "obj_id"
  ],
  "title": "Body"
}
```

### EmailObjectType

```json
{
  "type": "string",
  "enum": [
    "project",
    "project_comments",
    "task"
  ],
  "title": "EmailObjectType",
  "x-enumDescriptions": {
    "project": "Project",
    "project_comments": "Project Comments",
    "task": "Task"
  }
}
```

### _EmailResponse

```json
{
  "properties": {
    "email": {
      "type": "string",
      "title": "Email",
      "description": "Email address for the Todoist object.",
      "examples": [
        "add.task@example.todoist.net"
      ]
    }
  },
  "type": "object",
  "required": [
    "email"
  ],
  "title": "_EmailResponse"
}
```

### _StatusOkResponse

```json
{
  "properties": {
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
    "status"
  ],
  "title": "_StatusOkResponse"
}
```
