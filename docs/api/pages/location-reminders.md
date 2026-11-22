# Location reminders

_Availability of location reminders is dependent on the current user plan._

## Endpoints

### Delete Location Reminder

`DELETE /api/v1/location_reminders/{reminder_id}`

Delete a location reminder by ID.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the location reminder",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Reminder Id"
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

### Get Location Reminder

`GET /api/v1/location_reminders/{reminder_id}`

Return a single location reminder by ID.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the location reminder",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Reminder Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/NotificationLocationSyncView"
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

### Update Location Reminder

`POST /api/v1/location_reminders/{reminder_id}`

Update an existing location reminder.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the location reminder",
        "examples": [
          "6XGgmFQrx44wfGHr"
        ],
        "title": "Reminder Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_9c2f879b"
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
            "$ref": "#/components/schemas/NotificationLocationSyncView"
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

### Create Location Reminder

`POST /api/v1/location_reminders`

Create a new location reminder for a task.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_4c96330f"
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
            "$ref": "#/components/schemas/NotificationLocationSyncView"
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

### Get Location Reminders

`GET /api/v1/location_reminders`

Get all active location reminders.

Optionally filter by `task_id` to return only location reminders for a specific task.

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md)
for details on using cursor-based pagination.

```json
{
  "parameters": [
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
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PaginatedList_NotificationLocationSyncView_"
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

### Body_4c96330f

```json
{
  "properties": {
    "task_id": {
      "$ref": "#/components/schemas/LocationReminderTaskIDAttribute"
    },
    "name": {
      "type": "string",
      "maxLength": 255,
      "minLength": 1,
      "title": "Name",
      "description": "Name of the location",
      "examples": [
        "Home"
      ]
    },
    "loc_lat": {
      "type": "string",
      "maxLength": 20,
      "minLength": 1,
      "title": "Loc Lat",
      "description": "Latitude of the location",
      "examples": [
        "37.7749"
      ]
    },
    "loc_long": {
      "type": "string",
      "maxLength": 20,
      "minLength": 1,
      "title": "Loc Long",
      "description": "Longitude of the location",
      "examples": [
        "-122.4194"
      ]
    },
    "loc_trigger": {
      "type": "string",
      "enum": [
        "on_enter",
        "on_leave"
      ],
      "title": "Loc Trigger",
      "description": "Trigger type: 'on_enter' or 'on_leave'",
      "examples": [
        "on_enter"
      ]
    },
    "radius": {
      "type": "integer",
      "maximum": 255.0,
      "minimum": 1.0,
      "title": "Radius",
      "description": "Radius in meters (default 100, max 255)",
      "default": 100,
      "examples": [
        100
      ]
    },
    "notify_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Notify Uid",
      "description": "String ID of the user who receives the reminder. Must be a collaborator of the task's project. Defaults to the user creating the reminder. To find User IDs, use [Get user](https://developer.todoist.com/api/v1/user.md) for your own ID or [Get all collaborators](https://developer.todoist.com/api/v1/projects.md) for project members.",
      "examples": [
        "1234567"
      ]
    }
  },
  "type": "object",
  "required": [
    "task_id",
    "name",
    "loc_lat",
    "loc_long",
    "loc_trigger"
  ],
  "title": "Body"
}
```

### Body_9c2f879b

```json
{
  "properties": {
    "name": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 255,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Name",
      "description": "Name of the location. If not provided (or null), the value remains unchanged.",
      "examples": [
        "Office"
      ]
    },
    "loc_lat": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 20,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Loc Lat",
      "description": "Latitude of the location. If not provided (or null), the value remains unchanged.",
      "examples": [
        "40.416775"
      ]
    },
    "loc_long": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 20,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Loc Long",
      "description": "Longitude of the location. If not provided (or null), the value remains unchanged.",
      "examples": [
        "-3.703790"
      ]
    },
    "loc_trigger": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "on_enter",
            "on_leave"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Loc Trigger",
      "description": "Trigger type: 'on_enter' or 'on_leave'. If not provided (or null), the value remains unchanged.",
      "examples": [
        "on_enter"
      ]
    },
    "radius": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 255.0,
          "minimum": 1.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Radius",
      "description": "Radius in meters. If not provided (or null), the value remains unchanged.",
      "examples": [
        100
      ]
    },
    "notify_uid": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Notify Uid",
      "description": "String ID of the user who receives the reminder. Must be a collaborator of the task's project. To find User IDs, use [Get user](https://developer.todoist.com/api/v1/user.md) for your own ID or [Get all collaborators](https://developer.todoist.com/api/v1/projects.md) for project members. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "1234567"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### LocationReminderTaskIDAttribute

```json
{
  "type": "string",
  "description": "String ID of the task",
  "examples": [
    "6XGgmFVcrG5RRjVr"
  ]
}
```

### NotificationLocationSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the location reminder",
      "examples": [
        "6XGgmFQrx44wfGHr"
      ]
    },
    "item_id": {
      "type": "string",
      "title": "Item Id",
      "description": "String ID of the task",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "String ID of the project containing the task",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "notify_uid": {
      "type": "string",
      "title": "Notify Uid",
      "description": "String ID of the user who will receive the reminder",
      "examples": [
        "1029384756"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the location",
      "examples": [
        "Home"
      ]
    },
    "loc_lat": {
      "type": "string",
      "title": "Loc Lat",
      "description": "Latitude of the location",
      "examples": [
        "37.7749"
      ]
    },
    "loc_long": {
      "type": "string",
      "title": "Loc Long",
      "description": "Longitude of the location",
      "examples": [
        "-122.4194"
      ]
    },
    "loc_trigger": {
      "type": "string",
      "enum": [
        "on_enter",
        "on_leave"
      ],
      "title": "Loc Trigger",
      "description": "Trigger type for the location reminder",
      "examples": [
        "on_enter"
      ]
    },
    "radius": {
      "type": "integer",
      "title": "Radius",
      "description": "Radius in meters",
      "examples": [
        100
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the location reminder is deleted",
      "examples": [
        false
      ]
    },
    "type": {
      "type": "string",
      "const": "location",
      "title": "Type",
      "description": "The reminder type",
      "default": "location",
      "examples": [
        "location"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "item_id",
    "project_id",
    "notify_uid",
    "name",
    "loc_lat",
    "loc_long",
    "loc_trigger",
    "radius",
    "is_deleted"
  ],
  "title": "NotificationLocationSyncView",
  "description": "A class with fields representing a NotificationLocationView which will be\nreturned to clients in a sync-like response."
}
```

### PaginatedList_NotificationLocationSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/NotificationLocationSyncView"
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
