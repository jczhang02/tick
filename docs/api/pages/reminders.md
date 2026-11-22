# Reminders

_Availability of reminders is dependent on the current user plan._

## Endpoints

### Delete Reminder

`DELETE /api/v1/reminders/{reminder_id}`

Delete a reminder by ID.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the reminder",
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

### Get Reminder

`GET /api/v1/reminders/{reminder_id}`

Return a single reminder by ID.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the reminder",
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
            "$ref": "#/components/schemas/NotificationSyncView"
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

### Update Reminder

`POST /api/v1/reminders/{reminder_id}`

Update an existing reminder.

```json
{
  "parameters": [
    {
      "name": "reminder_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the reminder",
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
          "$ref": "#/components/schemas/Body_0d5e560b"
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
            "$ref": "#/components/schemas/NotificationSyncView"
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

### Create Reminder

`POST /api/v1/reminders`

Create a new reminder for a task.

For **relative** reminders, provide `minute_offset` (minutes before the task's due time).
For **absolute** reminders, provide a `due` dictionary with a specific date/time.
Supported `due` keys are `date` or `string`, with optional `lang`, `timezone`,
and `is_recurring`.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_958e83bd"
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
            "$ref": "#/components/schemas/NotificationSyncView"
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

### Get Reminders

`GET /api/v1/reminders`

Get all active reminders.

Optionally filter by `task_id` to return only reminders for a specific task.

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
            "$ref": "#/components/schemas/PaginatedList_NotificationSyncView_"
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

### Body_0d5e560b

```json
{
  "properties": {
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
      "description": "Minutes before due time to trigger (for relative reminders).",
      "examples": [
        30
      ]
    },
    "due": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ReminderDueAttribute"
        },
        {
          "type": "null"
        }
      ],
      "description": "Due date dictionary for absolute reminders.",
      "examples": [
        {
          "date": "2025-02-12T09:00:00",
          "lang": "en",
          "timezone": "Europe/Madrid"
        }
      ]
    },
    "service": {
      "$ref": "#/components/schemas/ReminderServiceType",
      "title": "Service",
      "description": "Notification service type: 'email', 'push'.",
      "examples": [
        "email"
      ]
    },
    "is_urgent": {
      "$ref": "#/components/schemas/UpdateReminderIsUrgentAttribute"
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

### Body_958e83bd

```json
{
  "properties": {
    "task_id": {
      "$ref": "#/components/schemas/ReminderTaskIDAttribute"
    },
    "reminder_type": {
      "type": "string",
      "enum": [
        "absolute",
        "relative"
      ],
      "title": "Reminder Type",
      "description": "Type of the reminder: 'relative' or 'absolute'",
      "default": "relative",
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
      "description": "Minutes before due time to trigger (for relative reminders)",
      "examples": [
        30
      ]
    },
    "due": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ReminderDueAttribute"
        },
        {
          "type": "null"
        }
      ],
      "description": "Due date dictionary for absolute reminders.",
      "examples": [
        {
          "date": "2025-02-12T09:00:00",
          "lang": "en",
          "timezone": "Europe/Madrid"
        }
      ]
    },
    "service": {
      "$ref": "#/components/schemas/ReminderServiceTypeAttribute",
      "title": "Service"
    },
    "is_urgent": {
      "$ref": "#/components/schemas/ReminderIsUrgentAttribute",
      "default": false
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
    "task_id"
  ],
  "title": "Body"
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

### PaginatedList_NotificationSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
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

### ReminderDueAttribute

```json
{
  "properties": {
    "date": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Date",
      "description": "Absolute reminder date in ISO 8601 format.",
      "examples": [
        "2025-02-12T09:00:00"
      ]
    },
    "string": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "String",
      "description": "Natural-language reminder date.",
      "examples": [
        "tomorrow at 9am"
      ]
    },
    "lang": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Lang",
      "description": "Language used to parse the natural-language date.",
      "examples": [
        "en"
      ]
    },
    "timezone": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Timezone",
      "description": "Timezone for the reminder date.",
      "examples": [
        "Europe/Madrid"
      ]
    },
    "is_recurring": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Recurring",
      "description": "Whether the reminder date is recurring.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "title": "ReminderDueAttribute",
  "description": "Due date input for absolute reminders."
}
```

### ReminderIsUrgentAttribute

```json
{
  "type": "boolean",
  "description": "Whether the reminder is an urgent reminder. Urgent reminders trigger an alarm notification on the user's designated urgent reminder device.",
  "examples": [
    false
  ]
}
```

### ReminderServiceType

```json
{
  "type": "string",
  "enum": [
    "email",
    "push"
  ]
}
```

### ReminderServiceTypeAttribute

```json
{
  "$ref": "#/components/schemas/ReminderServiceType",
  "description": "Notification service type: 'email', 'push'.",
  "examples": [
    "email"
  ]
}
```

### ReminderTaskIDAttribute

```json
{
  "type": "string",
  "description": "String ID of the task",
  "examples": [
    "6XGgmFVcrG5RRjVr"
  ]
}
```

### UpdateReminderIsUrgentAttribute

```json
{
  "type": "boolean",
  "description": "Whether the reminder is an urgent reminder. Urgent reminders trigger an alarm notification on the user's designated urgent reminder device. Omit this field to keep it unchanged.",
  "examples": [
    false
  ]
}
```
