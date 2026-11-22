# Activity

_Availability of the activity log and the duration of event storage are
dependent on the current user plan. These values are indicated by the
`activity_log` and `activity_log_limit` properties of the [user plan
limits](https://developer.todoist.com/api/v1/sync/user.md) object._

The activity log makes it easy to see everything that is happening across projects, items and notes.

The activity log uses the same **cursor-based pagination** as other endpoints in the API. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for general information about how cursor-based pagination works.

#### Logged events

Currently the official Todoist clients present only the most important events
that most users may be interested in.
There are further types of events related to projects, items and notes that are
stored in our database, and can be accessed through the API.

The following events are logged for items:

- Items added
- Items updated (only changes to `content`, `description`, `due_date` and `responsible_uid`)
- Items deleted
- Items completed
- Items uncompleted

The following events are logged for notes:

- Notes added
- Notes updated (only changes to `content` or `file_name` if the former is empty)
- Notes deleted

The following events are logged for projects:

- Projects added
- Projects updated (only changes to `name`)
- Projects deleted
- Projects archived
- Projects unarchived
- Projects shared
- Projects left

#### Pagination details

The activity log endpoint uses cursor-based pagination with the `cursor` and `limit` query parameters.

##### The `limit` parameter

Controls how many activity events to return per page.

- **Default**: 50
- **Maximum**: 200

##### The `cursor` parameter

An opaque token used to fetch the next page of results. Use the value from
the `next_cursor` field of a previous response. When `next_cursor` is `null`,
there are no more results to fetch.

##### Example

To fetch all activity events, start with a request without a cursor:

```bash
curl "https://api.todoist.com/api/v1/activities" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

The response will include a `results` array and a `next_cursor` value:

```json
{
  "results": [...],
  "next_cursor": "eyJwYWdlIjoyLCJsaW1pdCI6NTB9.aGFzaA"
}
```

To fetch the next page, pass the `next_cursor` value as the `cursor` parameter:

```bash
curl "https://api.todoist.com/api/v1/activities?cursor=eyJwYWdlIjoyLCJsaW1pdCI6NTB9.aGFzaA" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

Continue fetching pages until `next_cursor` is `null`.

**Note:** The `page` and `offset` parameters are not supported on this API version. 
Please refer to the [migration guide](https://developer.todoist.com/api/v1/migrating-from-v9.md) for more information.

## Endpoints

### Get Activity Logs

`GET /api/v1/activities`

Get activity logs.

Returns a paginated list of activity events for the user. Events can be filtered by object
type (project, item, note, section), event type, and other criteria. Uses cursor-based
pagination for efficient navigation through results.

```json
{
  "parameters": [
    {
      "name": "object_type",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "enum": [
              "project",
              "item",
              "note",
              "section"
            ],
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "The type of object to filter activities by. Must be one of \"project\", \"item\" (task), \"note\" (comment), or \"section\". When specified with `object_id`, returns activities for that specific object.",
        "examples": [
          "item"
        ],
        "title": "Object Type"
      }
    },
    {
      "name": "object_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer"
          },
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "The ID of the specific object to get activities for. Requires the object's type, either in `object_type` or as the only object type in `object_event_types`. For example, to get activities for a specific task, set `object_id=<task_id>` with `object_type=item` or `object_event_types=[\"item:\"]`.",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Object Id"
      }
    },
    {
      "name": "parent_project_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer"
          },
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities to only those belonging to the specified project. Returns activities for the project itself and all its tasks and comments.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Parent Project Id"
      }
    },
    {
      "name": "parent_item_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer"
          },
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities to only those belonging to the specified task. Returns activities for the task itself and all its comments.",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Parent Item Id"
      }
    },
    {
      "name": "include_parent_object",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "When `true` and `object_id` is specified, also include activities for the parent object. For example, when filtering by a specific task, also include activities for its parent project.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Include Parent Object"
      }
    },
    {
      "name": "include_child_objects",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "When `true` and `object_id` is specified, also include activities for all child objects. For example, when filtering by a project, also include activities for all its tasks and comments.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Include Child Objects"
      }
    },
    {
      "name": "initiator_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer"
          },
          {
            "type": "array",
            "items": {
              "type": "integer"
            }
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities to only those initiated by the specified user ID(s). Accepts either a single user ID or a list of user IDs. Useful for shared projects to see who made which changes.",
        "examples": [
          123456
        ],
        "title": "Initiator Id"
      }
    },
    {
      "name": "initiator_id_null",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "boolean"
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter by whether the activity has an initiator. When `true`, returns only activities with no initiator (your own activities). When `false`, returns only activities initiated by collaborators.",
        "examples": [
          false
        ],
        "title": "Initiator Id Null"
      }
    },
    {
      "name": "event_type",
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
        "description": "Filter by a simple event type (e.g., \"added\", \"deleted\", \"completed\"). Returns events of this type across ALL object types that support it. Valid types: \"added\", \"deleted\", \"updated\", \"archived\", \"unarchived\", \"completed\", \"uncompleted\", \"shared\", \"left\", \"moved\". Prefix the value with `!` to negate the filter and return all events EXCEPT this type (e.g., \"!deleted\"). For more precise filtering by both object type and event type, use `object_event_types` instead.",
        "examples": [
          "added",
          "deleted",
          "completed",
          "updated",
          "!deleted"
        ],
        "title": "Event Type"
      }
    },
    {
      "name": "ensure_last_state",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "**Deprecated** - This parameter has no implementation and will be removed in a future version.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Ensure Last State"
      }
    },
    {
      "name": "object_event_types",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "array",
            "items": {
              "type": "string"
            }
          },
          {
            "type": "null"
          }
        ],
        "description": "Advanced filtering for specific object type and event type combinations. Format: `[\"object_type:event_type\"]`. Examples: `[\"item:deleted\"]` for deleted tasks, `[\"item:\"]` for all task events, `[\":deleted\"]` for all delete events across all types, `[\"item:deleted\", \"note:added\"]` for multiple filters. Valid event types: \"added\", \"deleted\", \"updated\", \"completed\", \"uncompleted\", \"archived\", \"unarchived\", \"shared\", \"left\", \"moved\". This is the recommended way to filter events.",
        "examples": [
          [
            "item:deleted"
          ],
          [
            "item:",
            "note:added"
          ],
          [
            ":deleted"
          ]
        ],
        "title": "Object Event Types"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer"
          },
          {
            "type": "array",
            "items": {
              "anyOf": [
                {
                  "type": "integer"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities by workspace. Accepts a single workspace ID, a list of workspace IDs, or `null` to filter for personal projects (projects not belonging to any workspace). Use `[null, 123]` to include both personal projects and workspace 123. When not set, returns activities from all workspaces and personal projects.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    },
    {
      "name": "annotate_notes",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "When `true`, includes additional information about comments in the `extra_data` field, such as the content of the comment.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Annotate Notes"
      }
    },
    {
      "name": "annotate_parents",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "When `true`, includes additional information about parent objects in the `extra_data` field, such as the name of the parent project or task.",
        "examples": [
          false
        ],
        "default": false,
        "title": "Annotate Parents"
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
        "description": "Pagination cursor for fetching the next page of results. Use the value returned in the `next_cursor` field from a previous response.",
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
      "name": "date_from",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "format": "date-time"
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities to only those that occurred on or after this date. Must be in ISO 8601 format (e.g. '2026-01-01T00:00:00Z'). When specified, overrides the default pagination behavior and allows custom date ranges.",
        "examples": [
          "2026-01-01",
          "2026-01-01T00:00:00Z"
        ],
        "title": "Date From"
      }
    },
    {
      "name": "date_to",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "format": "date-time"
          },
          {
            "type": "null"
          }
        ],
        "description": "Filter activities to only those that occurred before this date (exclusive upper bound). Must be in ISO 8601 format (e.g. '2026-02-01T00:00:00Z'). When specified, overrides the default pagination behavior and allows custom date ranges.",
        "examples": [
          "2026-02-01T00:00:00Z"
        ],
        "title": "Date To"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PaginatedList_ActivityEvents_"
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

# Get all deleted tasks
$ curl --get https://api.todoist.com/api/v1/activities \
       -H "Authorization: Bearer $TOKEN" \
       -d object_event_types='["item:deleted"]'
```

```curl

# Get all task events (added, deleted, completed, uncompleted, etc.)
$ curl --get https://api.todoist.com/api/v1/activities \
       -H "Authorization: Bearer $TOKEN" \
       -d object_event_types='["item:"]'
```

```curl

# Get all deleted events (tasks, projects, notes)
$ curl --get https://api.todoist.com/api/v1/activities \
       -H "Authorization: Bearer $TOKEN" \
       -d object_event_types='[":deleted"]'
```

```curl

# Get deleted tasks and added notes
$ curl --get https://api.todoist.com/api/v1/activities \
       -H "Authorization: Bearer $TOKEN" \
       -d object_event_types='["item:deleted", "note:added"]'
```

## Schemas

The endpoints above refer to these schemas with `$ref`.

### ActivityEvents

```json
{
  "properties": {
    "object_type": {
      "$ref": "#/components/schemas/ActivityObjectType",
      "description": "The type of object this activity relates to. Valid values are `\"project\"`, `\"item\"`, `\"note\"`, or `\"section\"`.",
      "examples": [
        "item"
      ]
    },
    "object_id": {
      "type": "string",
      "title": "Object Id",
      "description": "The unique identifier of the object this activity relates to (project, item, note, or section ID).",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "event_type": {
      "type": "string",
      "title": "Event Type",
      "description": "The type of event that occurred. Valid values are: `\"added\"` (object was created), `\"deleted\"` (object was removed), `\"updated\"` (object was modified), `\"archived\"` (object was archived), `\"unarchived\"` (object was restored from archive), `\"completed\"` (task was completed), `\"uncompleted\"` (task was marked as incomplete), `\"shared\"` (project was shared with a user), `\"left\"` (user left a shared project), `\"moved\"` (task was moved between projects or between sections).",
      "examples": [
        "completed"
      ]
    },
    "event_date": {
      "type": "string",
      "format": "date-time",
      "title": "Event Date",
      "description": "The timestamp when this activity occurred.",
      "examples": [
        "2025-02-12T12:00:00Z"
      ]
    },
    "id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Id",
      "description": "The internal unique identifier for this activity log entry.",
      "examples": [
        123456789
      ]
    },
    "parent_project_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Parent Project Id",
      "description": "The ID of the project that contains the object this activity relates to. For project activities, this is the project itself. For item and note activities, this is the project containing the item or note.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "parent_item_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Parent Item Id",
      "description": "For note (comment) activities, the ID of the item (task) that the note is attached to. `null` for project and item activities.",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "initiator_id": {
      "anyOf": [
        {
          "type": "string",
          "description": "The ID of the user who is responsible for the event, which only makes sense in shared projects, items and notes, and is `null` for non-shared objects.",
          "examples": [
            "123456"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Initiator Id"
    },
    "extra_data_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Extra Data Id",
      "description": "Internal ID for the extra activity data row.",
      "examples": [
        123456789
      ]
    },
    "extra_data": {
      "anyOf": [
        {
          "additionalProperties": true,
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Extra Data",
      "description": "This object contains at least the `name` of the project, or the `content` of an item or comment, and optionally the `last_name` if a project was renamed, the `last_content` if an item or note was renamed, the `due_date` and `last_due_date` if an item's due date changed, the `responsible_uid` and `last_responsible_uid` if an item's responsible uid changed, the `description` and `last_description` if an item's description changed, the `priority` and `last_priority` if an item's priority changed, the `deadline` and `last_deadline` if an item's deadline changed, the `duration`, `last_duration`, `duration_unit`, and `last_duration_unit` if an item's duration changed, the `source_project_id` and `destination_project_id` if a task was moved between projects, the `section_id` on completion snapshots and section-move events, the `last_section_id` on section-move events indicating the previous section, and the `client` that caused the logging of the event",
      "examples": [
        {
          "content": "Buy milk"
        }
      ]
    }
  },
  "type": "object",
  "required": [
    "object_type",
    "object_id",
    "event_type",
    "event_date"
  ],
  "title": "ActivityEvents"
}
```

### ActivityObjectType

```json
{
  "type": "string",
  "enum": [
    "project",
    "item",
    "note",
    "section"
  ]
}
```

### PaginatedList_ActivityEvents_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/ActivityEvents"
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
