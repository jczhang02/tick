# Tasks

## Endpoints

### Tasks Completed By Completion Date

`GET /api/v1/tasks/completed/by_completion_date`

Retrieves a list of completed tasks strictly limited by the specified completion
date range (up to 3 months).

It can retrieve completed items:

- From all the projects the user has joined in a workspace
- From all the projects of the user
- That match many [supported
  filters](https://todoist.com/help/articles/introduction-to-filters-V98wIH)

By default, the response is limited to a page containing a maximum of 50 items
(configurable using `limit`).

Subsequent pages of results can be fetched by using the `next_cursor` value from the
response as the `cursor` value for the next request.

```json
{
  "parameters": [
    {
      "name": "since",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "format": "date-time",
        "description": "Start of the completion date range, inclusive.",
        "examples": [
          "2025-02-01T00:00:00Z"
        ],
        "title": "Since"
      }
    },
    {
      "name": "until",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "format": "date-time",
        "description": "End of the completion date range, exclusive.",
        "examples": [
          "2025-02-28T23:59:59Z"
        ],
        "title": "Until"
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
            "type": "null"
          }
        ],
        "description": "Workspace ID to limit completed tasks to.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
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
        "description": "Project ID to limit completed tasks to.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "section_id",
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
        "description": "Section ID to limit completed tasks to.",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    },
    {
      "name": "parent_id",
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
        "description": "Parent task ID to limit completed subtasks to.",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Parent Id"
      }
    },
    {
      "name": "filter_query",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "minLength": 1,
            "maxLength": 1024
          },
          {
            "type": "null"
          }
        ],
        "description": "Todoist filter query to limit completed tasks.",
        "examples": [
          "#Work & overdue"
        ],
        "title": "Filter Query"
      }
    },
    {
      "name": "filter_lang",
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
        "description": "Language code used to parse `filter_query`.",
        "examples": [
          "en"
        ],
        "title": "Filter Lang"
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
        "description": "Maximum number of completed tasks to return.",
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
            "$ref": "#/components/schemas/TasksCompletedDateResponse"
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

### Tasks Completed By Due Date

`GET /api/v1/tasks/completed/by_due_date`

Retrieves a list of completed items strictly limited by the specified due date range
(up to 6 weeks).

It can retrieve completed items:

- From within a project, section, or parent item
- From all the projects the user has joined in a workspace
- From all the projects of the user
- That match many [supported
  filters](https://todoist.com/help/articles/introduction-to-filters-V98wIH)

By default, the response is limited to a page containing a maximum of 50 items
(configurable using `limit`).

Subsequent pages of results can be fetched by using the `next_cursor` value from the
response as the `cursor` value for the next request.

```json
{
  "parameters": [
    {
      "name": "since",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "format": "date-time",
        "description": "Start of the due date range, inclusive.",
        "examples": [
          "2025-02-01T00:00:00Z"
        ],
        "title": "Since"
      }
    },
    {
      "name": "until",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "format": "date-time",
        "description": "End of the due date range, exclusive.",
        "examples": [
          "2025-02-28T23:59:59Z"
        ],
        "title": "Until"
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
            "type": "null"
          }
        ],
        "description": "Workspace ID to limit completed tasks to.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
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
        "description": "Project ID to limit completed tasks to.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "section_id",
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
        "description": "Section ID to limit completed tasks to.",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    },
    {
      "name": "parent_id",
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
        "description": "Parent task ID to limit completed subtasks to.",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Parent Id"
      }
    },
    {
      "name": "filter_query",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "minLength": 1,
            "maxLength": 1024
          },
          {
            "type": "null"
          }
        ],
        "description": "Todoist filter query to limit completed tasks.",
        "examples": [
          "#Work & overdue"
        ],
        "title": "Filter Query"
      }
    },
    {
      "name": "filter_lang",
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
        "description": "Language code used to parse `filter_query`.",
        "examples": [
          "en"
        ],
        "title": "Filter Lang"
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
        "description": "Maximum number of completed tasks to return.",
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
            "$ref": "#/components/schemas/TasksCompletedDateResponse"
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

### Get Tasks By Filter

`GET /api/v1/tasks/filter`

Get all tasks matching the filter.

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
        "description": "Filter by any [supported filter](https://todoist.com/help/articles/introduction-to-filters-V98wIH). Multiple filters (using the comma `,` operator) are not supported.",
        "examples": [
          "today | overdue"
        ],
        "title": "Query"
      }
    },
    {
      "name": "lang",
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
        "description": "IETF language tag defining what language filter is written in, if differs from default English",
        "examples": [
          "en",
          "de",
          "fr"
        ],
        "title": "Lang"
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
            "$ref": "#/components/schemas/PaginatedList_ItemSyncView_"
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

### Quick Add

`POST /api/v1/tasks/quick`

Add a new task using Quick Add with natural language processing.

Quick Add allows creating tasks with rich metadata using natural language text.
This endpoint uses the same parsing engine as Todoist's official clients,
supporting dates, projects, labels, priorities, assignees, and more in a single
text string.

#### Basic Syntax Overview

- **Due dates**: Natural language like `today`, `tomorrow at 5pm`, `next Monday`
- **Projects**: `#ProjectName` (use `#My\\ Project` for names with spaces)
- **Sections**: `/SectionName` (requires a project to be specified)
- **Labels**: `@labelname` (multiple labels supported)
- **Priority**: `p1` to `p4`, `P1` to `P4`, or `!!1` to `!!4`
- **Assignees**: `+Name` (requires a project to be specified)
- **Deadlines**: `{date expression}` (e.g., `{tomorrow}`, `{in 3 days}`)
- **Reminders**: `!30m` (30 min before), `!1h` (1 hour before), `!0mb` (at due time),
  `!14:00` (absolute time), `!30 min before` (verbose, English only)
- **Descriptions**: ` // description text` (must be at the end)

#### Examples

- `"Buy milk today at 5pm #Shopping @groceries p1"`
- `"Team meeting next Monday at 10am #Work +JohnDoe !30m"`
- `"Review PR {Friday} // Check the new authentication flow"`
- `"Submit report tomorrow #Work /Urgent @important // Include Q3 metrics"`

#### Important Notes

- Natural language date parsing supports multiple languages based on account settings
- If parsing fails for any element, the task is still created (with unparsed text
  remaining in the content)
- Use the `meta` parameter to see detailed parsing results for debugging

For comprehensive documentation including all supported date formats, language
support, and advanced syntax, see the
[Quick Add help article](https://www.todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz).

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_40158d1f"
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
                    "string": "tomorrow",
                    "lang": "en",
                    "is_recurring": false
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
              },
              "note": {
                "anyOf": [
                  {
                    "$ref": "#/components/schemas/ItemNoteSyncView"
                  },
                  {
                    "type": "null"
                  }
                ],
                "description": "Comment created from `note`. Only present when one was created."
              },
              "reminder": {
                "anyOf": [
                  {
                    "$ref": "#/components/schemas/NotificationSyncView"
                  },
                  {
                    "type": "null"
                  }
                ],
                "description": "Reminder created from `reminder`, the text or `auto_reminder`. Only present when one was created."
              },
              "meta": {
                "anyOf": [
                  {
                    "additionalProperties": true,
                    "type": "object"
                  },
                  {
                    "type": "null"
                  }
                ],
                "title": "Meta",
                "description": "How the text was parsed. Only present when `meta` is `true`."
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
            "title": "Response Quick Add Api V1 Tasks Quick Post",
            "description": "The created task, plus what else Quick Add created from the request. Only used for the docs."
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

### Reopen Task

`POST /api/v1/tasks/{task_id}/reopen`

Reopens a task.

Any ancestor tasks or sections will also be marked as uncomplete and
restored from history.

The reinstated tasks and sections will appear at the end of the list within
their parent, after any previously active tasks.

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
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

### Close Task

`POST /api/v1/tasks/{task_id}/close`

Closes a task.

The command performs in the same way as our official clients:

Regular tasks are marked complete and moved to history, along with their
subtasks. Tasks with [recurring due
dates](https://todoist.com/help/articles/introduction-to-recurring-dates-YUYVJJAV)
will be scheduled to their next occurrence.

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
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

### Move Task

`POST /api/v1/tasks/{task_id}/move`

Moves task to another project, section or parent.

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "ID of the task to move",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Task Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_02786f52"
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
            "$ref": "#/components/schemas/ItemSyncView"
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

### Delete Task

`DELETE /api/v1/tasks/{task_id}`

Delete a task and all of its subtasks.

Returns `NOT_FOUND` when the task does not exist and `FORBIDDEN` when the
authenticated user cannot modify the task.

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
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

### Get Task

`GET /api/v1/tasks/{task_id}`

Returns a single active (non-completed) task by ID

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
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
            "$ref": "#/components/schemas/ItemSyncView"
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

### Update Task

`POST /api/v1/tasks/{task_id}`

Updates an existing task.

```json
{
  "parameters": [
    {
      "name": "task_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the task",
        "examples": [
          "6XGgmFVcrG5RRjVr"
        ],
        "title": "Task Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_829067ab"
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
            "$ref": "#/components/schemas/ItemSyncView"
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

### Create Task

`POST /api/v1/tasks`

Create a new task.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_37565102"
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
            "$ref": "#/components/schemas/ItemSyncView"
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

### Get Tasks

`GET /api/v1/tasks`

Get all active tasks for the user.

All provided parameters are used to narrow down the list of tasks.

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
        "description": "String ID of the project to get tasks from",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "section_id",
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
        "description": "String ID of the section to get tasks from",
        "examples": [
          "6fFPHV272WWh3gpW"
        ],
        "title": "Section Id"
      }
    },
    {
      "name": "parent_id",
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
        "description": "String ID of the parent task to get sub-tasks from",
        "examples": [
          "6fFPHRxcmVqm4C84"
        ],
        "title": "Parent Id"
      }
    },
    {
      "name": "label",
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
        "description": "Filter tasks by label name.",
        "examples": [
          "next_action"
        ],
        "title": "Label"
      }
    },
    {
      "name": "ids",
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
        "description": "A list of the task IDs to retrieve, this should be a comma separated list",
        "examples": [
          "6XGgmFVcrG5RRjVr,6fFPHV272WWh3gpW"
        ],
        "title": "Ids"
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
            "$ref": "#/components/schemas/PaginatedList_ItemSyncView_"
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

### Body_02786f52

```json
{
  "properties": {
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
      "description": "ID of the project to move the task to",
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
      "description": "ID of the section to move the task to",
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
      "description": "ID of the parent task to move the task under",
      "examples": [
        "6fFPHRxcmVqm4C84"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_37565102

```json
{
  "properties": {
    "content": {
      "type": "string",
      "minLength": 1,
      "title": "Content",
      "description": "Task content.",
      "examples": [
        "Buy milk"
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
      "description": "Task description.",
      "examples": [
        "Pick up two liters of whole milk."
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
      "description": "ID of the project to add the task to. If omitted or null, the task will be added to the user's Inbox.",
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
      "description": "ID of the section to add the task to",
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
      "description": "ID of the parent task",
      "examples": [
        "6XGgmFVcrG5RRjVr"
      ]
    },
    "order": {
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
      "title": "Order",
      "description": "Position of the task in the project or section",
      "examples": [
        12
      ]
    },
    "labels": {
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
      "title": "Labels",
      "description": "List of label names.",
      "examples": [
        [
          "errands",
          "shopping"
        ]
      ]
    },
    "priority": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 4.0,
          "minimum": 1.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Priority",
      "description": "Task priority (1-4, where 1 is highest)",
      "examples": [
        2
      ]
    },
    "assignee_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Assignee Id",
      "description": "ID of the user to assign the task to. To find User IDs, use [Get user](https://developer.todoist.com/api/v1/user.md) for your own ID or [Get all collaborators](https://developer.todoist.com/api/v1/projects.md) for project members.",
      "examples": [
        123456789
      ]
    },
    "due_string": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due String",
      "description": "Human-readable representation of the due date. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.",
      "examples": [
        "tomorrow at 12:00"
      ]
    },
    "due_date": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due Date",
      "description": "Due date in RFC 3339 format or similar. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.",
      "examples": [
        "2025-02-12"
      ]
    },
    "due_datetime": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due Datetime",
      "description": "Due date and time. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.",
      "examples": [
        "2025-02-12T12:00:00Z"
      ]
    },
    "due_lang": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Due Lang",
      "description": "Due date language code. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.",
      "examples": [
        "en"
      ]
    },
    "duration": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Duration",
      "description": "Task duration, as a positive whole number of minutes or days. The API accepts durations longer than 24 hours, but the Todoist web and desktop date picker limits duration edits to 24 hours. Only used if `duration_unit` is also provided.",
      "examples": [
        30
      ]
    },
    "duration_unit": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "minute",
            "day"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Duration Unit",
      "description": "Unit of time for duration.",
      "examples": [
        "minute"
      ]
    },
    "deadline_date": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date",
      "title": "Deadline Date",
      "description": "Deadline date in YYYY-MM-DD format",
      "examples": [
        "2025-02-12"
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

### Body_40158d1f

```json
{
  "properties": {
    "text": {
      "type": "string",
      "title": "Text",
      "description": "The text of the task to parse using Quick Add syntax. See the endpoint description for supported syntax elements.",
      "examples": [
        "Buy milk today #Shopping @groceries p1",
        "Meeting tomorrow at 2pm #Work +Jane",
        "Call dentist next week !!1",
        "Review PR {Friday} // Check the authentication flow",
        "Submit report #Work /Urgent @important // Include Q3 metrics"
      ]
    },
    "note": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Note",
      "description": "A note to attach to the task. The note content is added as a comment on the created task. If the user's plan does not support comments, this parameter is ignored.",
      "examples": [
        "Remember to check the expiration date",
        "Meeting notes: Discuss Q4 goals"
      ]
    },
    "reminder": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Reminder",
      "description": "A reminder date in natural language format, similar to due date parsing. If the user's plan does not support reminders, this parameter is ignored. If the reminder text cannot be parsed, it is silently ignored.",
      "examples": [
        "tomorrow at 9am",
        "in 2 hours",
        "next Monday at 10am"
      ]
    },
    "auto_reminder": {
      "type": "boolean",
      "title": "Auto Reminder",
      "description": "When enabled, the default reminder will be added to the new item if it has a due date with time set. See also the [auto_reminder user option](https://developer.todoist.com/api/v1/sync/user.md) for more info about the default reminder.",
      "default": false,
      "examples": [
        false
      ]
    },
    "meta": {
      "type": "boolean",
      "title": "Meta",
      "description": "When enabled, the response includes a `meta` object containing the detailed parsing results. This shows exactly how the Quick Add text was interpreted, including which project, labels, due date, priority, etc. were detected. Useful for debugging parsing behavior or building integrations that need to understand how text was parsed.",
      "default": false,
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "text"
  ],
  "title": "Body"
}
```

### Body_829067ab

```json
{
  "properties": {
    "content": {
      "type": "string",
      "title": "Content",
      "description": "Updated task content. Omit this field to keep it unchanged.",
      "examples": [
        "Buy milk"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Updated task description. Omit this field to keep it unchanged.",
      "examples": [
        "Pick up two liters of whole milk."
      ]
    },
    "labels": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Labels",
      "description": "Updated list of label names. Omit this field to keep it unchanged.",
      "examples": [
        [
          "errands",
          "shopping"
        ]
      ]
    },
    "priority": {
      "type": "integer",
      "maximum": 4.0,
      "minimum": 1.0,
      "title": "Priority",
      "description": "Updated task priority (1-4, where 1 is highest). Omit this field to keep it unchanged.",
      "examples": [
        2
      ]
    },
    "due_string": {
      "type": "string",
      "title": "Due String",
      "description": "Updated human-readable representation of the due date. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details. Omit this field to keep it unchanged.",
      "examples": [
        "tomorrow at 12:00"
      ]
    },
    "due_date": {
      "type": "string",
      "title": "Due Date",
      "description": "Updated due date in RFC 3339 format or similar. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details. Omit this field to keep it unchanged.",
      "examples": [
        "2025-02-12"
      ]
    },
    "due_datetime": {
      "type": "string",
      "title": "Due Datetime",
      "description": "Updated due date and time. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details. Omit this field to keep it unchanged.",
      "examples": [
        "2025-02-12T12:00:00Z"
      ]
    },
    "due_lang": {
      "type": "string",
      "title": "Due Lang",
      "description": "Updated due date language code. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details. Omit this field to keep it unchanged.",
      "examples": [
        "en"
      ]
    },
    "assignee_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Assignee Id",
      "description": "ID of the user to assign the task to. To find User IDs, use [Get user](https://developer.todoist.com/api/v1/user.md) for your own ID or [Get all collaborators](https://developer.todoist.com/api/v1/projects.md) for project members. Pass null to clear the value. Omit this field to keep it unchanged.",
      "examples": [
        123456789
      ]
    },
    "duration": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Duration",
      "description": "Updated task duration, as a positive whole number of minutes or days. The API accepts durations longer than 24 hours, but the Todoist web and desktop date picker limits duration edits to 24 hours. Only used if `duration_unit` is also provided. Pass null to clear the value. Omit this field to keep it unchanged.",
      "examples": [
        30
      ]
    },
    "duration_unit": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "minute",
            "day"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Duration Unit",
      "description": "Unit of time for duration. Must be provided to update the task duration. Pass null to clear the value. Omit this field to keep it unchanged.",
      "examples": [
        "minute"
      ]
    },
    "deadline_date": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date",
      "title": "Deadline Date",
      "description": "Updated deadline date in YYYY-MM-DD format. Pass null to clear the value. Omit this field to keep it unchanged.",
      "examples": [
        "2025-02-12"
      ]
    },
    "child_order": {
      "type": "integer",
      "maximum": 2147483647.0,
      "minimum": -2147483648.0,
      "title": "Child Order",
      "description": "Updated position of the task in its current scope. Omit this field to keep it unchanged.",
      "examples": [
        12
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Updated collapsed state of the task for the current user. Omit this field to keep it unchanged.",
      "examples": [
        false
      ]
    },
    "day_order": {
      "type": "integer",
      "maximum": 2147483647.0,
      "minimum": -2147483648.0,
      "title": "Day Order",
      "description": "Updated position of the task in Today and Upcoming views. Omit this field to keep it unchanged.",
      "examples": [
        3
      ]
    }
  },
  "type": "object",
  "title": "Body"
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

### PaginatedList_ItemSyncView_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
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

### TasksCompletedDateResponse

```json
{
  "properties": {
    "items": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Items",
      "description": "Completed tasks in the current page.",
      "examples": [
        [
          {
            "added_at": "2025-01-15T10:30:00Z",
            "added_by_uid": "1234567",
            "assigned_by_uid": "1234567",
            "checked": false,
            "child_order": 1,
            "completed_at": "2025-01-16T10:30:00Z",
            "completed_by_uid": "1234567",
            "completed_count": 3,
            "content": "Buy milk",
            "day_order": 1,
            "deadline": {
              "date": "2025-02-12",
              "lang": "en"
            },
            "description": "Pick up organic milk",
            "due": {
              "date": "2025-02-12",
              "is_recurring": false,
              "lang": "en",
              "string": "tomorrow"
            },
            "duration": {
              "amount": 30,
              "unit": "minute"
            },
            "id": "6XGgmFVcrG5RRjVr",
            "is_collapsed": false,
            "is_deleted": false,
            "labels": [
              "priority"
            ],
            "note_count": 0,
            "order_key": "a1V",
            "parent_id": "6XGgmFVcrG5RRjVr",
            "postponed_count": 1,
            "priority": 1,
            "project_id": "6XGgm6PHrGgMpCFX",
            "responsible_uid": "1234567",
            "section_id": "6fFPHV272WWh3gpW",
            "updated_at": "2025-01-17T10:30:00Z",
            "user_id": "1234567"
          }
        ]
      ]
    },
    "next_cursor": {
      "type": "string",
      "title": "Next Cursor",
      "description": "Cursor for the next page of completed tasks, when more results are available. Omitted when there is no next page.",
      "examples": [
        "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
      ]
    }
  },
  "type": "object",
  "title": "TasksCompletedDateResponse"
}
```
