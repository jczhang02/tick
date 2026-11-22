# User

## Endpoints

### Get Productivity Stats

`GET /api/v1/tasks/completed/stats`

Get comprehensive productivity statistics for the authenticated user.
This endpoint is used to display information about the
[Karma system](https://www.todoist.com/help/articles/introduction-to-karma-OgWkWy).

Returns detailed completion statistics including:

- Daily completion counts with per-project breakdowns for the last 7 days
- Weekly completion counts with per-project breakdowns for the last 4 weeks
- Total completed task count
- Karma score, trend, graph data, and update history
- Goal settings (daily/weekly goals, ignore days, vacation mode)
- Streak information (current, last, and maximum daily and weekly streaks)
- Project color mappings for visualization

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/ProductivityStatsResponse"
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

### Update Notification Setting

`PUT /api/v1/notification_setting`

Update a notification delivery preference.

The API accepts the legacy `dont_notify` flag and stores the inverse as the
user's delivery preference for the requested notification type and channel.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_604e3e52"
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
            "additionalProperties": {
              "additionalProperties": {
                "type": "boolean"
              },
              "type": "object"
            },
            "type": "object",
            "title": "Response Update Notification Setting Api V1 Notification Setting Put",
            "description": "Notification delivery preferences keyed by notification type.",
            "examples": [
              {
                "user_left_project": {
                  "notify_push": true,
                  "notify_email": true
                }
              }
            ]
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

### User Info

`GET /api/v1/user`

Get information about the currently authenticated user.

This is the OIDC userinfo endpoint. OAuth tokens of any product audience
are accepted as long as they carry the `user:read` scope (implied by
`data:read`).

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/UserJSON"
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

### Body_604e3e52

```json
{
  "properties": {
    "notification_type": {
      "$ref": "#/components/schemas/NotificationType",
      "description": "Notification type to update.",
      "examples": [
        "workspace_invitation_created"
      ]
    },
    "service": {
      "$ref": "#/components/schemas/NotificationChannel",
      "description": "Notification channel to update.",
      "examples": [
        "email"
      ]
    },
    "dont_notify": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Dont Notify",
      "description": "Whether notifications should be disabled for the selected type and channel.",
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "required": [
    "notification_type",
    "service"
  ],
  "title": "Body"
}
```

### DailyCompletionItem

```json
{
  "properties": {
    "date": {
      "type": "string",
      "format": "date",
      "title": "Date",
      "description": "Date for this daily completion data (ISO 8601 format: YYYY-MM-DD)",
      "examples": [
        "2025-02-12"
      ]
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ProjectCompletedItem"
      },
      "type": "array",
      "title": "Items",
      "description": "Per-project completion breakdown for this day",
      "examples": [
        [
          {
            "completed": 3,
            "id": "6XGgm6PHrGgMpCFX"
          }
        ]
      ]
    },
    "total_completed": {
      "type": "integer",
      "title": "Total Completed",
      "description": "Total number of tasks completed across all projects on this day",
      "examples": [
        3
      ]
    }
  },
  "type": "object",
  "required": [
    "date",
    "items",
    "total_completed"
  ],
  "title": "DailyCompletionItem",
  "description": "Completion statistics for a single day."
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

### GoalsSettings

```json
{
  "properties": {
    "user": {
      "type": "string",
      "title": "User",
      "description": "User ID (legacy field, same as user_id)",
      "examples": [
        "1234567"
      ]
    },
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "User ID",
      "examples": [
        "1234567"
      ]
    },
    "daily_goal": {
      "type": "integer",
      "title": "Daily Goal",
      "description": "Daily task completion goal set by the user",
      "examples": [
        5
      ]
    },
    "weekly_goal": {
      "type": "integer",
      "title": "Weekly Goal",
      "description": "Weekly task completion goal set by the user",
      "examples": [
        25
      ]
    },
    "ignore_days": {
      "items": {
        "type": "integer"
      },
      "type": "array",
      "title": "Ignore Days",
      "description": "List of weekday numbers (0=Sunday, 1=Monday, ..., 6=Saturday) excluded from daily goals",
      "examples": [
        [
          6,
          0
        ]
      ]
    },
    "vacation_mode": {
      "type": "integer",
      "title": "Vacation Mode",
      "description": "Vacation mode status (0=disabled, 1=enabled)",
      "examples": [
        0
      ]
    },
    "karma_disabled": {
      "type": "integer",
      "title": "Karma Disabled",
      "description": "Karma tracking status (0=enabled, 1=disabled)",
      "examples": [
        0
      ]
    },
    "current_daily_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Current consecutive daily completion streak",
      "examples": [
        {
          "count": 5,
          "end": "2025-02-14",
          "start": "2025-02-10"
        }
      ]
    },
    "current_weekly_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Current consecutive weekly completion streak",
      "examples": [
        {
          "count": 2,
          "end": "2025-02-16",
          "start": "2025-02-03"
        }
      ]
    },
    "last_daily_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Previous daily streak (before the current one)",
      "examples": [
        {
          "count": 3,
          "end": "2025-01-22",
          "start": "2025-01-20"
        }
      ]
    },
    "last_weekly_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Previous weekly streak (before the current one)",
      "examples": [
        {
          "count": 1,
          "end": "2025-01-26",
          "start": "2025-01-20"
        }
      ]
    },
    "max_daily_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Longest daily streak achieved by the user",
      "examples": [
        {
          "count": 10,
          "end": "2025-01-10",
          "start": "2025-01-01"
        }
      ]
    },
    "max_weekly_streak": {
      "$ref": "#/components/schemas/StreakInfo",
      "description": "Longest weekly streak achieved by the user",
      "examples": [
        {
          "count": 4,
          "end": "2025-01-28",
          "start": "2025-01-01"
        }
      ]
    }
  },
  "type": "object",
  "required": [
    "user",
    "user_id",
    "daily_goal",
    "weekly_goal",
    "ignore_days",
    "vacation_mode",
    "karma_disabled",
    "current_daily_streak",
    "current_weekly_streak",
    "last_daily_streak",
    "last_weekly_streak",
    "max_daily_streak",
    "max_weekly_streak"
  ],
  "title": "GoalsSettings",
  "description": "User goals and settings."
}
```

### GraphPoint

```json
{
  "properties": {
    "date": {
      "type": "string",
      "title": "Date",
      "description": "Date for the karma data point in YYYY-MM-DD format.",
      "examples": [
        "2025-02-12"
      ]
    },
    "karma_avg": {
      "type": "integer",
      "title": "Karma Avg",
      "description": "Average karma score for this date.",
      "examples": [
        1024
      ]
    }
  },
  "type": "object",
  "required": [
    "date",
    "karma_avg"
  ],
  "title": "GraphPoint",
  "description": "Historical karma data point for graphing."
}
```

### KarmaUpdateReasonEntry

```json
{
  "properties": {
    "time": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Time",
      "description": "Timestamp when this karma update occurred (ISO 8601 datetime)",
      "examples": [
        "2025-02-12T10:30:00Z"
      ]
    },
    "new_karma": {
      "type": "number",
      "title": "New Karma",
      "description": "New total karma score after this update",
      "examples": [
        1024.0
      ]
    },
    "positive_karma": {
      "type": "number",
      "title": "Positive Karma",
      "description": "Amount of karma gained in this update",
      "examples": [
        1.0
      ]
    },
    "negative_karma": {
      "type": "number",
      "title": "Negative Karma",
      "description": "Amount of karma lost in this update (negative value)",
      "examples": [
        0.0
      ]
    },
    "positive_karma_reasons": {
      "items": {
        "$ref": "#/components/schemas/PositiveKarmaReason"
      },
      "type": "array",
      "title": "Positive Karma Reasons",
      "description": "List of reason codes for karma gains. Values: 1=Tasks added, 2=Tasks completed, 3=Advanced features, 4=Signup, 5=Beta upgrade, 6=Support activity, 7=Premium upgrade, 8=Getting started, 9=Daily goal reached, 10=Weekly goal reached",
      "examples": [
        [
          2,
          9
        ]
      ]
    },
    "negative_karma_reasons": {
      "items": {
        "$ref": "#/components/schemas/NegativeKarmaReason"
      },
      "type": "array",
      "title": "Negative Karma Reasons",
      "description": "List of reason codes for karma losses. Values: 50=Tasks overdue, 51=Tasks postponed, 52=Is inactive",
      "examples": [
        [
          50
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "new_karma",
    "positive_karma",
    "negative_karma",
    "positive_karma_reasons",
    "negative_karma_reasons"
  ],
  "title": "KarmaUpdateReasonEntry",
  "description": "Karma update event with reasons and amounts."
}
```

### NegativeKarmaReason

```json
{
  "type": "integer",
  "enum": [
    50,
    51,
    52
  ],
  "title": "NegativeKarmaReason",
  "description": "Reasons why karma can decrease.",
  "x-enumDescriptions": {
    "50": "Tasks Overdue",
    "51": "Tasks Postponed",
    "52": "Is Inactive"
  }
}
```

### NotificationChannel

```json
{
  "type": "string",
  "enum": [
    "email",
    "push"
  ],
  "title": "NotificationChannel",
  "description": "Which communication mechanism is being used to send this notification",
  "x-enumDescriptions": {
    "email": "Email",
    "push": "Push"
  }
}
```

### NotificationType

```json
{
  "type": "string",
  "enum": [
    "note_added",
    "item_assigned",
    "item_completed",
    "item_uncompleted",
    "karma_level",
    "share_invitation_sent",
    "share_invitation_accepted",
    "share_invitation_rejected",
    "share_invitation_blocked_by_project_limit",
    "user_left_project",
    "user_removed_from_project",
    "teams_workspace_upgraded",
    "teams_workspace_canceled",
    "teams_workspace_payment_failed",
    "pro_trial_started",
    "pro_trial_ended",
    "workspace_invitation_created",
    "workspace_invitation_accepted",
    "workspace_invitation_rejected",
    "project_archived",
    "project_moved",
    "removed_from_workspace",
    "workspace_deleted",
    "message",
    "workspace_user_joined_by_domain",
    "price_increase_new_pro_users",
    "price_increase_new_team",
    "price_increase_new_team_trial",
    "price_increase_android",
    "workspace_team_cohort_tagged"
  ],
  "title": "NotificationType",
  "description": "The type of notification being sent",
  "x-enumDescriptions": {
    "item_assigned": "Item Assigned",
    "item_completed": "Item Completed",
    "item_uncompleted": "Item Uncompleted",
    "karma_level": "Karma Level",
    "message": "Message",
    "note_added": "Note Added",
    "price_increase_android": "Price Increase Android",
    "price_increase_new_pro_users": "Price Increase New Pro Users",
    "price_increase_new_team": "Price Increase New Team",
    "price_increase_new_team_trial": "Price Increase New Team Trial",
    "pro_trial_ended": "Pro Trial Ended",
    "pro_trial_started": "Pro Trial Started",
    "project_archived": "Project Archived",
    "project_moved": "Project Moved",
    "removed_from_workspace": "Removed From Workspace",
    "share_invitation_accepted": "Share Invitation Accepted",
    "share_invitation_blocked_by_project_limit": "Share Invitation Blocked By Project Limit",
    "share_invitation_rejected": "Share Invitation Rejected",
    "share_invitation_sent": "Share Invitation Sent",
    "teams_workspace_canceled": "Teams Workspace Canceled",
    "teams_workspace_payment_failed": "Teams Workspace Payment Failed",
    "teams_workspace_upgraded": "Teams Workspace Upgraded",
    "user_left_project": "User Left Project",
    "user_removed_from_project": "User Removed From Project",
    "workspace_deleted": "Workspace Deleted",
    "workspace_invitation_accepted": "Workspace Invitation Accepted",
    "workspace_invitation_created": "Workspace Invitation Created",
    "workspace_invitation_rejected": "Workspace Invitation Rejected",
    "workspace_team_cohort_tagged": "Workspace Team Acquisition Cohort Tagged",
    "workspace_user_joined_by_domain": "Workspace User Joined By Domain"
  }
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

### PositiveKarmaReason

```json
{
  "type": "integer",
  "enum": [
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10
  ],
  "title": "PositiveKarmaReason",
  "description": "Reasons why karma can increase.",
  "x-enumDescriptions": {
    "1": "Tasks Added",
    "2": "Tasks Completed",
    "3": "Advanced Features",
    "4": "Signup",
    "5": "Beta Upgrade",
    "6": "Support Activity",
    "7": "Premium Upgrade",
    "8": "Getting Started",
    "9": "Daily Goal Reached",
    "10": "Weekly Goal Reached"
  }
}
```

### ProductivityStatsResponse

```json
{
  "properties": {
    "days_items": {
      "items": {
        "$ref": "#/components/schemas/DailyCompletionItem"
      },
      "type": "array",
      "title": "Days Items",
      "description": "Daily completion statistics for the last 7 days, ordered chronologically",
      "examples": [
        [
          {
            "date": "2025-02-12",
            "items": [],
            "total_completed": 3
          }
        ]
      ]
    },
    "week_items": {
      "items": {
        "$ref": "#/components/schemas/WeeklyCompletionItem"
      },
      "type": "array",
      "title": "Week Items",
      "description": "Weekly completion statistics for the last 4 weeks, ordered chronologically. Only available for Pro and Business plan users",
      "examples": [
        [
          {
            "from": "2025-02-10",
            "items": [],
            "to": "2025-02-16",
            "total_completed": 12
          }
        ]
      ]
    },
    "project_colors": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object",
      "title": "Project Colors",
      "description": "Mapping of project IDs to their color names (e.g., {'2aB3cD4eF5gH6iJ7': 'blue', '8kL9mN0oP1qR2sT3': 'red'})",
      "examples": [
        {
          "6XGgm6PHrGgMpCFX": "blue"
        }
      ]
    },
    "completed_count": {
      "type": "integer",
      "title": "Completed Count",
      "description": "Total number of tasks completed by the user (all time)",
      "examples": [
        42
      ]
    },
    "karma": {
      "type": "number",
      "title": "Karma",
      "description": "Current karma score",
      "examples": [
        1024.0
      ]
    },
    "karma_trend": {
      "type": "string",
      "title": "Karma Trend",
      "description": "Karma trend indicator ('up' or 'down')",
      "examples": [
        "up"
      ]
    },
    "karma_graph_data": {
      "items": {
        "$ref": "#/components/schemas/GraphPoint"
      },
      "type": "array",
      "title": "Karma Graph Data",
      "description": "Historical karma data points for graphing, each containing date and karma_avg",
      "examples": [
        [
          {
            "date": "2025-02-12",
            "karma_avg": 1024
          }
        ]
      ]
    },
    "karma_last_update": {
      "type": "number",
      "title": "Karma Last Update",
      "description": "Net karma change from the most recent update (positive_karma + negative_karma)",
      "examples": [
        1.0
      ]
    },
    "karma_update_reasons": {
      "items": {
        "$ref": "#/components/schemas/KarmaUpdateReasonEntry"
      },
      "type": "array",
      "title": "Karma Update Reasons",
      "description": "Recent karma update events with reasons and amounts",
      "examples": [
        [
          {
            "negative_karma": 0.0,
            "negative_karma_reasons": [
              50
            ],
            "new_karma": 1024.0,
            "positive_karma": 1.0,
            "positive_karma_reasons": [
              2,
              9
            ],
            "time": "2025-02-12T10:30:00Z"
          }
        ]
      ]
    },
    "goals": {
      "$ref": "#/components/schemas/GoalsSettings",
      "description": "User goals, streaks, and productivity settings"
    }
  },
  "type": "object",
  "required": [
    "days_items",
    "week_items",
    "project_colors",
    "completed_count",
    "karma",
    "karma_trend",
    "karma_graph_data",
    "karma_last_update",
    "karma_update_reasons",
    "goals"
  ],
  "title": "ProductivityStatsResponse",
  "description": "Comprehensive productivity statistics response.\n\nIncludes completion counts, karma information, goals, and streaks."
}
```

### ProjectCompletedItem

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "Project ID (v1 or v2 format depending on API version)",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "completed": {
      "type": "integer",
      "title": "Completed",
      "description": "Number of tasks completed in this project during the time period",
      "examples": [
        3
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "completed"
  ],
  "title": "ProjectCompletedItem",
  "description": "Completion statistics for a specific project."
}
```

### StreakInfo

```json
{
  "properties": {
    "count": {
      "type": "integer",
      "title": "Count",
      "description": "Number of consecutive days/weeks in the streak",
      "examples": [
        5
      ]
    },
    "start": {
      "anyOf": [
        {
          "type": "string",
          "format": "date"
        },
        {
          "type": "null"
        }
      ],
      "title": "Start",
      "description": "Start date of the streak (ISO 8601 format: YYYY-MM-DD), or null if no streak",
      "examples": [
        "2025-02-10"
      ]
    },
    "end": {
      "anyOf": [
        {
          "type": "string",
          "format": "date"
        },
        {
          "type": "null"
        }
      ],
      "title": "End",
      "description": "End date of the streak (ISO 8601 format: YYYY-MM-DD), or null if no streak",
      "examples": [
        "2025-02-14"
      ]
    }
  },
  "type": "object",
  "required": [
    "count"
  ],
  "title": "StreakInfo",
  "description": "Streak information (count and dates)."
}
```

### ThemeAppearance

```json
{
  "type": "string",
  "enum": [
    "light",
    "dark",
    "system"
  ],
  "title": "ThemeAppearance",
  "x-enumDescriptions": {
    "dark": "Dark",
    "light": "Light",
    "system": "System"
  }
}
```

### UrgentReminderDeviceView

```json
{
  "properties": {
    "device_platform": {
      "type": "string",
      "enum": [
        "ios",
        "android"
      ],
      "title": "Device Platform",
      "description": "Registered device platform.",
      "examples": [
        "ios"
      ]
    },
    "device_id": {
      "type": "string",
      "title": "Device Id",
      "description": "Registered device ID.",
      "examples": [
        "device-123"
      ]
    },
    "device_token": {
      "type": "string",
      "title": "Device Token",
      "description": "Registered device token.",
      "examples": [
        "token-123"
      ]
    },
    "device_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Device Name",
      "description": "Registered device name when available.",
      "examples": [
        "Maria's iPhone"
      ]
    }
  },
  "type": "object",
  "required": [
    "device_platform",
    "device_id",
    "device_token"
  ],
  "title": "UrgentReminderDeviceView"
}
```

### UserJSON

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "User ID",
      "examples": [
        "1234567"
      ]
    },
    "email": {
      "type": "string",
      "title": "Email",
      "description": "User's email address",
      "examples": [
        "user@example.com"
      ]
    },
    "full_name": {
      "type": "string",
      "title": "Full Name",
      "description": "The user's real name formatted as Firstname Lastname",
      "examples": [
        "Maria Garcia"
      ]
    },
    "has_password": {
      "type": "boolean",
      "title": "Has Password",
      "description": "Whether the user has a password set on the account. It will be false if they have only authenticated without a password (e.g. using Google, Facebook, etc.)",
      "examples": [
        true
      ]
    },
    "verification_status": {
      "type": "string",
      "enum": [
        "unverified",
        "verified",
        "blocked",
        "legacy"
      ],
      "title": "Verification Status",
      "description": "User's email verification status. unverified (just signed up), verified (verified email or social login), blocked (failed to verify in 7 days), legacy (signed up before August 2022)",
      "examples": [
        "verified"
      ]
    },
    "mfa_enabled": {
      "type": "boolean",
      "title": "Mfa Enabled",
      "description": "Whether multi-factor authentication is enabled",
      "examples": [
        false
      ]
    },
    "token": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Token",
      "description": "The user's token that should be used to call the other API methods",
      "examples": [
        "0123456789abcdef0123456789abcdef01234567"
      ]
    },
    "is_premium": {
      "type": "boolean",
      "title": "Is Premium",
      "description": "Whether the user has a Todoist Pro subscription (a true or false value)",
      "examples": [
        true
      ]
    },
    "premium_status": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "not_premium",
            "current_personal_plan",
            "legacy_personal_plan",
            "teams_business_member"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Premium Status",
      "description": "Outlines why a user is premium, possible values are: not_premium, current_personal_plan, legacy_personal_plan or teams_business_member",
      "examples": [
        "current_personal_plan"
      ]
    },
    "premium_until": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Premium Until",
      "description": "The date when the user's Todoist Pro subscription ends (null if not a Todoist Pro user). This should be used for informational purposes only as this does not include the grace period upon expiration",
      "examples": [
        "2025-12-31T23:59:59Z"
      ]
    },
    "free_trial_expires": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Free Trial Expires",
      "description": "Date when free trial expires (ISO 8601 format)",
      "examples": [
        "2025-03-01T00:00:00Z"
      ]
    },
    "has_started_a_trial": {
      "type": "boolean",
      "title": "Has Started A Trial",
      "description": "Whether the user has ever started a free trial",
      "examples": [
        true
      ]
    },
    "joined_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Joined At",
      "description": "Date when user joined Todoist (ISO 8601 format)",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the user is deleted",
      "default": false,
      "examples": [
        false
      ]
    },
    "deleted_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Deleted At",
      "description": "Date when user was deleted (ISO 8601 format)",
      "examples": [
        null
      ]
    },
    "business_account_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Business Account Id",
      "description": "The ID of the user's business account",
      "examples": [
        null
      ]
    },
    "date_format": {
      "type": "integer",
      "enum": [
        0,
        1
      ],
      "title": "Date Format",
      "description": "Whether to use the DD-MM-YYYY date format (if set to 0), or the MM-DD-YYYY format (if set to 1)",
      "examples": [
        1
      ]
    },
    "time_format": {
      "anyOf": [
        {
          "type": "integer",
          "enum": [
            0,
            1
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Time Format",
      "description": "Whether to use a 24h format such as 13:00 (if set to 0) when displaying time, or a 12h format such as 1:00pm (if set to 1)",
      "examples": [
        0
      ]
    },
    "sort_order": {
      "type": "integer",
      "enum": [
        0,
        1
      ],
      "title": "Sort Order",
      "description": "Whether to show projects in an oldest dates first order (if set to 0), or a oldest dates last order (if set to 1)",
      "examples": [
        0
      ]
    },
    "theme_id": {
      "type": "string",
      "title": "Theme Id",
      "description": "The currently selected Todoist theme (a number between 0 and 13)",
      "examples": [
        "todoist"
      ]
    },
    "theme_appearance": {
      "$ref": "#/components/schemas/ThemeAppearance",
      "description": "Whether Todoist uses a light theme, a dark theme, or follows the system appearance",
      "examples": [
        "system"
      ]
    },
    "start_day": {
      "type": "integer",
      "maximum": 7.0,
      "minimum": 1.0,
      "title": "Start Day",
      "description": "The first day of the week (between 1 and 7, where 1 is Monday and 7 is Sunday)",
      "examples": [
        1
      ]
    },
    "weekend_start_day": {
      "type": "integer",
      "maximum": 7.0,
      "minimum": 1.0,
      "title": "Weekend Start Day",
      "description": "The day used when a user chooses to schedule a task for the 'Weekend' (between 1 and 7, where 1 is Monday and 7 is Sunday)",
      "examples": [
        6
      ]
    },
    "next_week": {
      "type": "integer",
      "maximum": 7.0,
      "minimum": 1.0,
      "title": "Next Week",
      "description": "The day of the next week, that tasks will be postponed to (between 1 and 7, where 1 is Monday and 7 is Sunday)",
      "examples": [
        1
      ]
    },
    "auto_reminder": {
      "type": "integer",
      "title": "Auto Reminder",
      "description": "The default time in minutes for the automatic reminders set, whenever a due date has been specified for a task",
      "examples": [
        30
      ]
    },
    "urgent_reminder_device": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/UrgentReminderDeviceView"
        },
        {
          "type": "null"
        }
      ],
      "description": "The device that should ring urgent reminders. Contains device_id, device_token, device_platform (ios or android), and optionally device_name.",
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
      "description": "The user's status, shared across Todoist and Comms. `null` when the user has none or it has expired. Set or clear it with `user_update`.",
      "examples": [
        null
      ]
    },
    "start_page": {
      "type": "string",
      "title": "Start Page",
      "description": "The user's default view on Todoist. The start page can be one of the following: inbox, teaminbox, today, next7days, upcoming, assigned_to_me, project?id=1234 to open a project, label?name=abc to open a label, filter?id=1234 to open a filter, or workspace_filter?id=1234 to open a workspace filter",
      "examples": [
        "inbox"
      ]
    },
    "inbox_project_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Inbox Project Id",
      "description": "The ID of the user's Inbox project",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "lang": {
      "type": "string",
      "enum": [
        "cs",
        "da",
        "de",
        "en",
        "es",
        "fi",
        "fr",
        "it",
        "ja",
        "ko",
        "nl",
        "pl",
        "pt_BR",
        "ru",
        "sv",
        "tr",
        "zh_CN",
        "zh_TW"
      ],
      "title": "Lang",
      "description": "The user's language",
      "examples": [
        "en"
      ]
    },
    "tz_info": {
      "additionalProperties": true,
      "type": "object",
      "title": "Tz Info",
      "description": "The user's timezone (a dictionary structure), which includes the following elements: the timezone as a string value, the hours and minutes difference from GMT, whether daylight saving time applies denoted by is_dst, and a string value of the time difference from GMT that is gmt_string",
      "examples": [
        {
          "gmt_string": "+01:00",
          "is_dst": false,
          "timezone": "Europe/Madrid"
        }
      ]
    },
    "karma": {
      "type": "number",
      "title": "Karma",
      "description": "The user's karma score",
      "examples": [
        1024.0
      ]
    },
    "karma_trend": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "up",
            "down",
            "-"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Karma Trend",
      "description": "The user's karma trend. Can be 'up', 'down', or '-' (no change)",
      "examples": [
        "up"
      ]
    },
    "daily_goal": {
      "type": "integer",
      "title": "Daily Goal",
      "description": "The daily goal number of completed tasks for karma",
      "examples": [
        5
      ]
    },
    "weekly_goal": {
      "type": "integer",
      "title": "Weekly Goal",
      "description": "The target number of tasks to complete per week",
      "examples": [
        25
      ]
    },
    "days_off": {
      "items": {
        "type": "integer"
      },
      "type": "array",
      "title": "Days Off",
      "description": "Array of integers representing user's days off (between 1 and 7, where 1 is Monday and 7 is Sunday)",
      "examples": [
        [
          6,
          7
        ]
      ]
    },
    "is_celebrations_enabled": {
      "type": "boolean",
      "title": "Is Celebrations Enabled",
      "description": "Whether celebration animations are enabled",
      "examples": [
        true
      ]
    },
    "completed_count": {
      "type": "integer",
      "title": "Completed Count",
      "description": "Total number of tasks completed by user",
      "examples": [
        42
      ]
    },
    "completed_today": {
      "type": "integer",
      "title": "Completed Today",
      "description": "Number of tasks completed today by the user",
      "examples": [
        3
      ]
    },
    "share_limit": {
      "type": "integer",
      "title": "Share Limit",
      "description": "Maximum number of collaborators allowed in shared projects",
      "examples": [
        5
      ]
    },
    "features": {
      "additionalProperties": true,
      "type": "object",
      "title": "Features",
      "description": "Feature flags and settings for the user",
      "examples": [
        {
          "webhooks": true
        }
      ]
    },
    "feature_identifier": {
      "type": "string",
      "title": "Feature Identifier",
      "description": "Feature identifier for feature flag evaluations",
      "examples": [
        "1234567"
      ]
    },
    "joinable_workspace": {
      "anyOf": [
        {
          "additionalProperties": true,
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Joinable Workspace",
      "description": "Information about workspaces the user can join",
      "examples": [
        null
      ]
    },
    "onboarding_completed": {
      "type": "boolean",
      "title": "Onboarding Completed",
      "description": "Whether the user has completed onboarding",
      "examples": [
        true
      ]
    },
    "onboarding_initiated": {
      "type": "boolean",
      "title": "Onboarding Initiated",
      "description": "Whether the user has initiated onboarding",
      "examples": [
        true
      ]
    },
    "onboarding_started": {
      "type": "boolean",
      "title": "Onboarding Started",
      "description": "Whether the user has started onboarding",
      "examples": [
        true
      ]
    },
    "onboarding_level": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "beginner",
            "intermediate",
            "pro"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Level",
      "description": "User's self-reported skill level during onboarding",
      "examples": [
        "intermediate"
      ]
    },
    "onboarding_persona": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "analog",
            "tasks",
            "calendar",
            "organic"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Persona",
      "description": "User's onboarding persona selection",
      "examples": [
        "tasks"
      ]
    },
    "onboarding_role": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "leader",
            "founder",
            "ic"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Role",
      "description": "User's role selection during onboarding",
      "examples": [
        "ic"
      ]
    },
    "onboarding_team_mode": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Team Mode",
      "description": "Whether user selected team mode during onboarding",
      "examples": [
        "solo"
      ]
    },
    "onboarding_use_cases": {
      "anyOf": [
        {
          "items": {
            "type": "string",
            "enum": [
              "personal",
              "work",
              "education",
              "teamwork",
              "solo",
              "teamcreator",
              "simple",
              "teamjoiner"
            ]
          },
          "type": "array"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Use Cases",
      "description": "Use cases the user selected during onboarding",
      "examples": [
        [
          "work"
        ]
      ]
    },
    "getting_started_guide_projects": {
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
      "title": "Getting Started Guide Projects",
      "description": "List of project IDs for getting started guide",
      "examples": [
        [
          "6XGgm6PHrGgMpCFX"
        ]
      ]
    },
    "onboarded_dated_tasks_created": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarded Dated Tasks Created",
      "description": "True once the user has created at least 1 task with a due date that includes a time component (excluding auto-seeded getting-started-guide tasks).",
      "examples": [
        true
      ]
    },
    "onboarded_tasks_completed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarded Tasks Completed",
      "description": "True once the user has completed at least 3 tasks.",
      "examples": [
        true
      ]
    },
    "onboarded_mobile_accessed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarded Mobile Accessed",
      "description": "True once the user has synced from iOS or Android.",
      "examples": [
        true
      ]
    },
    "onboarded_desktop_accessed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarded Desktop Accessed",
      "description": "True once the user has synced from Web, macOS, or Windows.",
      "examples": [
        true
      ]
    },
    "onboarded_desktop_skipped": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarded Desktop Skipped",
      "description": "True once the user skips Get Todoist on your computer.",
      "examples": [
        false
      ]
    },
    "is_onboarded": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Onboarded",
      "description": "True once the user completes the onboarding card: both task criteria are met, the user has accessed mobile, and has either accessed desktop or skipped the desktop step.",
      "examples": [
        true
      ]
    },
    "activated_user": {
      "type": "boolean",
      "title": "Activated User",
      "description": "Whether the user is considered activated (completed key onboarding actions)",
      "examples": [
        true
      ]
    },
    "has_magic_number": {
      "type": "boolean",
      "title": "Has Magic Number",
      "description": "Whether the user has reached a magic number milestone",
      "examples": [
        false
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
      "description": "The ID of the user's avatar",
      "examples": [
        "abc123"
      ]
    },
    "avatar_big": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Avatar Big",
      "description": "The link to a 195x195 pixels image of the user's avatar",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/avatar_big.jpg"
      ]
    },
    "avatar_medium": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Avatar Medium",
      "description": "The link to a 60x60 pixels image of the user's avatar",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/avatar_medium.jpg"
      ]
    },
    "avatar_s640": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Avatar S640",
      "description": "The link to a 640x640 pixels image of the user's avatar",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/avatar_s640.jpg"
      ]
    },
    "avatar_small": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Avatar Small",
      "description": "The link to a 35x35 pixels image of the user's avatar",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/avatar_small.jpg"
      ]
    },
    "websocket_url": {
      "type": "string",
      "title": "Websocket Url",
      "description": "WebSocket URL for real-time updates",
      "examples": [
        "wss://ws.todoist.com/ws?token=0123456789abcdef"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "email",
    "full_name",
    "has_password",
    "verification_status",
    "mfa_enabled",
    "is_premium",
    "has_started_a_trial",
    "date_format",
    "sort_order",
    "theme_id",
    "theme_appearance",
    "start_day",
    "weekend_start_day",
    "next_week",
    "auto_reminder",
    "start_page",
    "lang",
    "tz_info",
    "karma",
    "daily_goal",
    "weekly_goal",
    "days_off",
    "is_celebrations_enabled",
    "completed_count",
    "completed_today",
    "share_limit",
    "features",
    "feature_identifier",
    "joinable_workspace",
    "onboarding_completed",
    "onboarding_initiated",
    "onboarding_started",
    "activated_user",
    "has_magic_number",
    "websocket_url"
  ],
  "title": "UserSyncView",
  "description": "A model for the user objects returned by the sync/REST API.\n\nThis model is used purely for OpenAPI documentation generation and does not\nhandle actual serialization (which is handled by @todoist.models.json.user.generate)."
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

### WeeklyCompletionItem

```json
{
  "properties": {
    "from": {
      "type": "string",
      "format": "date",
      "title": "From",
      "description": "Start date of the week (Monday, ISO 8601 format: YYYY-MM-DD)",
      "examples": [
        "2025-02-10"
      ]
    },
    "to": {
      "type": "string",
      "format": "date",
      "title": "To",
      "description": "End date of the week (Sunday, ISO 8601 format: YYYY-MM-DD)",
      "examples": [
        "2025-02-16"
      ]
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ProjectCompletedItem"
      },
      "type": "array",
      "title": "Items",
      "description": "Per-project completion breakdown for this week",
      "examples": [
        [
          {
            "completed": 12,
            "id": "6XGgm6PHrGgMpCFX"
          }
        ]
      ]
    },
    "total_completed": {
      "type": "integer",
      "title": "Total Completed",
      "description": "Total number of tasks completed across all projects during this week",
      "examples": [
        12
      ]
    }
  },
  "type": "object",
  "required": [
    "from",
    "to",
    "items",
    "total_completed"
  ],
  "title": "WeeklyCompletionItem",
  "description": "Completion statistics for a week."
}
```
