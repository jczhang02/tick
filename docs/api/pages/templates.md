# Templates

Templates allow exporting of a project's tasks to a file or URL, and then
importing of the task list to a new or existing project.

Availability of project templates functionality is dependent on the current
user plan. This values is indicated by the `templates` property of the [user
plan limits](https://developer.todoist.com/api/v1/sync/user.md) object.

Workspace-owned templates retain task assignments only to current admins,
members and guests of that workspace. Saving replacement content or moving a
personal template into a workspace clears ineligible task assignees and their
assignment actors. Comment authors and reminder recipients are preserved.
When a person leaves a workspace, their task assignments in its saved templates
are cleared asynchronously. If they rejoin before the cleanup runs, their
assignments are kept.

### Preview a template

Use `GET /api/v1/templates/{template_id}/preview` for a curated slug or a saved
template ID beginning with `UT_`. Curated previews allow anonymous access. Saved
templates require `data:read` and retain their personal or workspace visibility
rules. `locale` selects curated content. For setup templates, `for_workspace=true`
previews workspace projects and folders.

Use `GET /api/v1/templates/preview?project_id=...` to preview an existing project.
This requires `data:read` and permission to export that project.

Previews resolve dates by default. For project templates, `resolve_dates=false`
returns raw due, deadline and absolute-reminder strings, such as
`{"string":"in 3 days","lang":"en"}`, for use in a template editor. Project
previews also include the calendar snapshot, reminders and comment attachments
in `notes[].file_attachment` and `project_notes[].file_attachment`. Project IDs
in a preview are temporary graph references. Setup templates reject
`resolve_dates=false`.

### Use a template

`POST /api/v1/templates/{template_id}/use` creates a project or a complete setup
and requires `data:read_write`. It accepts `locale` and an optional destination
`workspace_id`. A saved workspace template can only be used in its associated
workspace. Project templates also accept `folder_id`, which requires
`workspace_id`. Setup templates reject `folder_id`.

The response contains the created graph using the existing Sync API names:
tasks are `items`, task comments are `notes`, and project comments are
`project_notes`. `project_ids` contains objects shaped as `{"project_id":"..."}`.
The graph includes reminders and user view options. Project use returns empty
label and filter lists. This response does not include `discarded_fields` or
`project_view_options_defaults`.

### Export a project

`GET /api/v1/templates/file?project_id=...` requires `data:read` and returns UTF-8
CSV bytes with a download filename. `GET /api/v1/templates/url?project_id=...`
requires `data:read_write` because it publishes a stored, publicly accessible CSV
artifact. No expiry is set on the returned URL. Both routes retain project export
permissions and plan checks, and accept `use_relative_dates` (default `true`).

## Endpoints

### Preview User Template

`POST /api/v1/templates/user/preview`

Stages and previews a project CSV sent as multipart `file` or a raw CSV body.

Requires `data:read_write` because the upload is stored. The response includes
the filename for a subsequent import and the graph for editing; project names
come from the filename or the `X-File-Name` header for raw bodies.

```json
{
  "parameters": [
    {
      "name": "resolve_dates",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads.",
      "schema": {
        "type": "boolean",
        "title": "Resolve Dates",
        "description": "Resolve due dates and deadlines. Use false for an editor to receive raw date strings for the supported content graph projection. On v1.6+, raw typed dues also include date_type, time and timezone to preserve their meaning when saved.",
        "default": true,
        "examples": [
          false
        ]
      }
    }
  ],
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_preview_user_template_api_v1_templates_user_preview_post"
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
            "$ref": "#/components/schemas/PreviewFromFileResponse"
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

### Import User Template From File

`POST /api/v1/templates/user/import`

Creates a personal template from CSV or a content graph. Requires `data:read_write`.

Task assignees and assignment actors are discarded from personal templates.

With `content` unset, the request's file (either a multipart `file` or a
previously-uploaded `uploaded_file_name`, or a raw CSV body) becomes the
template's content. Send content graphs in an `application/json` body.
With `content` set, an uploaded file is ignored and `uploaded_file_name` must be unset.
Import creates a personal template; update it to associate a workspace.

```json
{
  "parameters": [
    {
      "name": "template_type",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads. Required for raw uploads.",
      "schema": {
        "type": "string",
        "const": "project",
        "title": "Template Type",
        "description": "Saved templates support project content only.",
        "examples": [
          "project"
        ]
      }
    },
    {
      "name": "name",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads. Required for raw uploads.",
      "schema": {
        "type": "string",
        "maxLength": 255,
        "title": "Name",
        "description": "Saved template name, up to 255 characters.",
        "examples": [
          "Weekly planning"
        ]
      }
    },
    {
      "name": "description",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads. Required for raw uploads.",
      "schema": {
        "type": "string",
        "maxLength": 1000,
        "title": "Description",
        "description": "Saved template description, up to 1,000 characters.",
        "examples": [
          "Plan and review the week."
        ]
      }
    },
    {
      "name": "color",
      "in": "query",
      "required": false,
      "description": "Send in the query string for raw uploads, or the form for multipart uploads. Required for raw uploads.",
      "schema": {
        "$ref": "#/components/schemas/APIColorID",
        "description": "Template color.",
        "examples": [
          "berry_red"
        ]
      }
    },
    {
      "name": "uploaded_file_name",
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
        "title": "Uploaded File Name",
        "description": "Staged filename from /templates/user/preview, owned by the caller. Mutually exclusive with content.",
        "examples": [
          "1234567_abcdef_weekly-planning.csv"
        ]
      }
    }
  ],
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_import_user_template_from_file_api_v1_templates_user_import_post"
        }
      },
      "application/json": {
        "schema": {
          "properties": {
            "template_type": {
              "type": "string",
              "const": "project",
              "title": "Template Type",
              "description": "Saved templates support project content only.",
              "examples": [
                "project"
              ]
            },
            "name": {
              "type": "string",
              "maxLength": 255,
              "title": "Name",
              "description": "Saved template name, up to 255 characters.",
              "examples": [
                "Weekly planning"
              ]
            },
            "description": {
              "type": "string",
              "maxLength": 1000,
              "title": "Description",
              "description": "Saved template description, up to 1,000 characters.",
              "examples": [
                "Plan and review the week."
              ]
            },
            "color": {
              "$ref": "#/components/schemas/APIColorID",
              "description": "Template color.",
              "examples": [
                "berry_red"
              ]
            },
            "uploaded_file_name": {
              "anyOf": [
                {
                  "type": "string"
                },
                {
                  "type": "null"
                }
              ],
              "title": "Uploaded File Name",
              "description": "Staged filename from /templates/user/preview, owned by the caller. Mutually exclusive with content.",
              "examples": [
                "1234567_abcdef_weekly-planning.csv"
              ]
            },
            "content": {
              "anyOf": [
                {
                  "$ref": "#/components/schemas/TemplateContent"
                },
                {
                  "type": "null"
                }
              ],
              "title": "Content",
              "description": "Full replacement graph. Uses the supported writable fields of a raw preview, including optional calendar defaults. IDs refer only to nodes in this payload. Mutually exclusive with uploaded_file_name. When content and a multipart file are supplied, content takes precedence and the file is ignored."
            }
          },
          "type": "object",
          "required": [
            "template_type",
            "name",
            "description",
            "color"
          ]
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
            "$ref": "#/components/schemas/CreateUserTemplateResponse"
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

### Import Into Project From Template Id

`POST /api/v1/templates/import_into_project_from_template_id`

Import a saved template into an existing project.

The target project must exist and must not be frozen. The response includes
the imported objects so clients can update local state immediately.
Tasks imported from personal templates are unassigned.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_925d2d9b"
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
            "$ref": "#/components/schemas/ProjectImportResponse"
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

### Import Into Project From File

`POST /api/v1/templates/import_into_project_from_file`

A template can be imported in an existing project, or in a newly created one.

Upload a file suitable to be passed as a template to be imported into a project.

```json
{
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_import_into_project_from_file_api_v1_templates_import_into_project_from_file_post"
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
            "$ref": "#/components/schemas/ProjectImportResponse"
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

$ curl https://api.todoist.com/api/v1/templates/import_into_project_from_file \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -F project_id=6XGgm6PHrGgMpCFX \
       -F file=@example.csv
```

### Create Project From File

`POST /api/v1/templates/create_project_from_file`

A template can be imported in an existing project, or in a newly created one.

Upload a file suitable to be passed as a template to be imported into a project.

```json
{
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_create_project_from_file_api_v1_templates_create_project_from_file_post"
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

$ curl https://api.todoist.com/api/v1/templates/create_project_from_file \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -F name=My Project \
       -F file=@example.csv
```

### Generate Description

`POST /api/v1/templates/generate_description`

Generates an AI description from a project or saved template. Supply `project_id`
or `template_id`. Requires `data:read_write`, access to the source and the AI template descriptions feature.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_11f87528"
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
            "$ref": "#/components/schemas/GenerateDescriptionResponse"
          }
        }
      }
    },
    "429": {
      "description": "AI description service rate limit exceeded"
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

### Categories

`GET /api/v1/templates/categories`

List curated template categories. Authentication is not required.

```json
{
  "parameters": [
    {
      "name": "template_type",
      "in": "query",
      "required": false,
      "schema": {
        "enum": [
          "setup",
          "project",
          "all"
        ],
        "type": "string",
        "description": "Template type to categorize.",
        "examples": [
          "project"
        ],
        "default": "project",
        "title": "Template Type"
      }
    },
    {
      "name": "locale",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Locale for category names and descriptions. Unsupported locales fall back to English.",
        "examples": [
          "en"
        ],
        "default": "en",
        "title": "Locale"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/CategoriesResponse"
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

### Preview Project

`GET /api/v1/templates/preview`

Preview a project's template graph. Requires `data:read` and project export permission.

Set `resolve_dates=false` to preserve raw date strings for a template editor.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "description": "ID of the project to preview as a template.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "resolve_dates",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Resolve date strings to concrete dates. Set to false to preserve raw due, deadline and absolute-reminder date strings when editing a project template.",
        "examples": [
          true
        ],
        "default": true,
        "title": "Resolve Dates"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/ProjectTemplatePreviewResponse"
          }
        }
      }
    },
    "429": {
      "description": "Request rate limit exceeded"
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

### Update User Template

`PUT /api/v1/templates/user/{template_id}`

Updates a template's metadata and, optionally, its content.

Task assignments in replacement content are retained only for workspace templates.

Requires `data:read_write` and template edit permission. Omitted metadata stays
unchanged. Supply `content` or `project_id` to replace the graph. A shared workspace
template can be edited by its author or a workspace admin; restricted templates
remain author-only, and only the author may change sharing.

If workspace membership repeatedly changes during saving, retry the same
request after the 409 `WORKSPACE_TEMPLATE_MEMBERSHIP_CHANGED` response.

```json
{
  "parameters": [
    {
      "name": "template_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "ID of a saved personal or workspace template.",
        "examples": [
          "UT_28Rc62VC22j439VcPJMw9xgWx2"
        ],
        "title": "Template Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_93c42499"
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
            "$ref": "#/components/schemas/UpdateUserTemplateResponse"
          }
        }
      }
    },
    "409": {
      "description": "Workspace membership changed; retry the same template save"
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

### Export As File

`GET /api/v1/templates/file`

Download a project as UTF-8 CSV. Requires `data:read` and project export permission.

On v1.6+, dated tasks also get `DATE_TYPE` and `DATE_TIME` columns, and
`TIMEZONE` holds the task's own timezone only for `fixed_datetime` dues.
Older versions write the exporting user's timezone on every row. With
`use_relative_dates=false`, non-recurring dues are ISO 8601 values.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project to export as a template.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "use_relative_dates",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Whether to export due dates as relative dates.",
        "examples": [
          true
        ],
        "default": true,
        "title": "Use Relative Dates"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "text/plain": {
          "schema": {
            "type": "string"
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

### Create User Template

`POST /api/v1/templates/user`

Creates a template from a project or a content graph. Requires `data:read_write`.

Task assignments are retained only when the resulting template belongs to a workspace.

`project_id` is always required. With `content` unset, its content is
exported as-is. With `content` set, project_id is only used to check
access, set the template's source project, and (with `for_workspace`)
derive the workspace to associate the template with - the content graph
is used for the template's content instead of an export of the project.

If workspace membership repeatedly changes during saving, retry the same
request after the 409 `WORKSPACE_TEMPLATE_MEMBERSHIP_CHANGED` response.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_d730dae6"
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
            "$ref": "#/components/schemas/CreateUserTemplateResponse"
          }
        }
      }
    },
    "409": {
      "description": "Workspace membership changed; retry the same template save"
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

### Export As Url

`GET /api/v1/templates/url`

Publish a project as a shareable template URL. Requires `data:read_write` and project export permission.

This stores a publicly accessible CSV artifact. No expiry is set on this URL.
Pass it to `https://todoist.com/api/v1/import/project_from_url?t_url=<url>` to share the template.

On v1.6+, dated tasks also get `DATE_TYPE` and `DATE_TIME` columns, and
`TIMEZONE` holds the task's own timezone only for `fixed_datetime` dues.
Older versions write the exporting user's timezone on every row. With
`use_relative_dates=false`, non-recurring dues are ISO 8601 values.

```json
{
  "parameters": [
    {
      "name": "project_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "description": "String ID of the project to export as a template URL.",
        "examples": [
          "6XGgm6PHrGgMpCFX"
        ],
        "title": "Project Id"
      }
    },
    {
      "name": "use_relative_dates",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Whether to export due dates as relative dates.",
        "examples": [
          true
        ],
        "default": true,
        "title": "Use Relative Dates"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/FileURLResponse"
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

### Preview Template

`GET /api/v1/templates/{template_id}/preview`

Preview a project or setup template without importing it.

Curated templates support anonymous access and use a valid `data:read` actor when available.
Any `UT_` ID requires `data:read` and the saved template's existing visibility permissions.
Raw date strings are supported only for project templates.

```json
{
  "parameters": [
    {
      "name": "template_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "Curated template slug or saved template ID with a `UT_` prefix.",
        "examples": [
          "reading-list"
        ],
        "title": "Template Id"
      }
    },
    {
      "name": "locale",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "maxLength": 10,
        "description": "Locale for curated template content, with the template's English fallback.",
        "examples": [
          "en"
        ],
        "default": "en",
        "title": "Locale"
      }
    },
    {
      "name": "for_workspace",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Preview a setup as workspace projects and folders, rather than personal projects.",
        "examples": [
          false
        ],
        "default": false,
        "title": "For Workspace"
      }
    },
    {
      "name": "resolve_dates",
      "in": "query",
      "required": false,
      "schema": {
        "type": "boolean",
        "description": "Resolve date strings to concrete dates. Set to false to preserve raw due, deadline and absolute-reminder date strings when editing a project template.",
        "examples": [
          true
        ],
        "default": true,
        "title": "Resolve Dates"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/TemplatePreviewResponse"
          }
        }
      }
    },
    "429": {
      "description": "Request rate limit exceeded"
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

### Use Template

`POST /api/v1/templates/{template_id}/use`

Create a project or a complete setup from a template. Requires `data:read_write`.

Saved workspace templates can only be used in their associated workspace.
Returns the created sync-style graph, with tasks in `items` and comments in `notes`.

```json
{
  "parameters": [
    {
      "name": "template_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "Curated template slug or saved template ID with a `UT_` prefix.",
        "examples": [
          "reading-list"
        ],
        "title": "Template Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_37d1a6e4"
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
            "$ref": "#/components/schemas/CreateFromTemplateIdResponse"
          }
        }
      }
    },
    "429": {
      "description": "Request rate limit exceeded"
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

### Discover Template

`GET /api/v1/templates/{template_id}`

Get one template. A `UT_` ID requires `data:read`; invisible or missing IDs return 404.

```json
{
  "parameters": [
    {
      "name": "template_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "A curated template slug or a saved template ID prefixed with `UT_`.",
        "examples": [
          "bug-tracking",
          "UT_28Rc62VC22j439VcPJMw9xgWx2"
        ],
        "title": "Template Id"
      }
    },
    {
      "name": "locale",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Content locale. Unsupported locales fall back to English.",
        "examples": [
          "en"
        ],
        "default": "en",
        "title": "Locale"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PublicTemplate"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate-limiter cache temporarily unavailable"
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

### Discover Templates

`GET /api/v1/templates`

Discover templates or look up a bounded batch of IDs.

Curated templates are anonymous. Personal/workspace sources and any `UT_` ID
require `data:read`. Pages prioritize personal, workspace, then curated
templates, and rank each page by the query. Keep all filters when continuing.

```json
{
  "parameters": [
    {
      "name": "template_type",
      "in": "query",
      "required": false,
      "schema": {
        "enum": [
          "setup",
          "project",
          "all"
        ],
        "type": "string",
        "description": "Template type to list.",
        "examples": [
          "project"
        ],
        "default": "project",
        "title": "Template Type"
      }
    },
    {
      "name": "template_source",
      "in": "query",
      "required": false,
      "schema": {
        "enum": [
          "todoist",
          "user",
          "workspace",
          "all"
        ],
        "type": "string",
        "description": "Curated (`todoist`), personal (`user`), workspace, or all accessible templates.",
        "examples": [
          "todoist"
        ],
        "default": "todoist",
        "title": "Template Source"
      }
    },
    {
      "name": "locale",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Content locale. Unsupported locales fall back to English.",
        "examples": [
          "en"
        ],
        "default": "en",
        "title": "Locale"
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
      "name": "query",
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
        "description": "Search template text and, without category_id, matching curated categories.",
        "examples": [
          "work"
        ],
        "title": "Query"
      }
    },
    {
      "name": "category_id",
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
        "description": "Curated category slug. Personal and workspace templates retain their own search filtering.",
        "examples": [
          "work"
        ],
        "title": "Category Id"
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
        "description": "Up to 100 comma-separated ID entries. Repeated IDs are deduplicated in request order. Missing or invisible IDs are omitted. May be combined with locale and pagination, but not listing filters.",
        "examples": [
          "bug-tracking,UT_28Rc62VC22j439VcPJMw9xgWx2"
        ],
        "title": "Ids"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/PaginatedList_PublicTemplate_"
          }
        }
      }
    },
    "429": {
      "description": "Rate limit exceeded"
    },
    "503": {
      "description": "Rate-limiter cache temporarily unavailable"
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

### Body_11f87528

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
      "description": "Project to summarize. Takes precedence when template_id is also supplied.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "template_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Template Id",
      "description": "Saved template to summarize.",
      "examples": [
        "UT_28Rc62VC22j439VcPJMw9xgWx2"
      ]
    },
    "locale": {
      "type": "string",
      "title": "Locale",
      "description": "Language for the generated description.",
      "default": "en",
      "examples": [
        "en"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_37d1a6e4

```json
{
  "properties": {
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
      "description": "Destination workspace ID. Omit to create personal projects.",
      "examples": [
        "1234567"
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
      "description": "Destination folder for a project template. Requires workspace_id. Unsupported for setups.",
      "examples": [
        "1234568"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_925d2d9b

```json
{
  "properties": {
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "String ID of the project to import the template into.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "template_id": {
      "type": "string",
      "title": "Template Id",
      "description": "ID of the saved template to import.",
      "examples": [
        "123456"
      ]
    },
    "locale": {
      "type": "string",
      "title": "Locale",
      "description": "Locale to use for localized template content.",
      "default": "en",
      "examples": [
        "en"
      ]
    }
  },
  "type": "object",
  "required": [
    "project_id",
    "template_id"
  ],
  "title": "Body"
}
```

### Body_93c42499

```json
{
  "properties": {
    "name": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 255,
          "description": "Saved template name, up to 255 characters.",
          "examples": [
            "Weekly planning"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Name"
    },
    "description": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 1000,
          "description": "Saved template description, up to 1,000 characters.",
          "examples": [
            "Plan and review the week."
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Description"
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
      "description": "Replace content by exporting this project. Mutually exclusive with content.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
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
      "description": "Template color. Omit to keep it unchanged.",
      "examples": [
        "berry_red"
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
      "description": "Associate a personal template with this workspace, keeping its ID. Workspace templates cannot move to another workspace.",
      "examples": [
        "1234567"
      ]
    },
    "shared_with_workspace_members": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Shared With Workspace Members",
      "description": "Share or unshare with workspace members. Only the author can change this setting. Omit to keep it unchanged.",
      "examples": [
        true
      ]
    },
    "content": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/TemplateContent"
        },
        {
          "type": "null"
        }
      ],
      "title": "Content",
      "description": "Full replacement graph. Uses the supported writable fields of a raw preview, including optional calendar defaults. IDs refer only to nodes in this payload. Mutually exclusive with project_id. Omit to keep the content unchanged."
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_create_project_from_file_api_v1_templates_create_project_from_file_post

```json
{
  "properties": {
    "name": {
      "type": "string",
      "title": "Name"
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
      "title": "Workspace Id"
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
  "title": "Body_create_project_from_file_api_v1_templates_create_project_from_file_post"
}
```

### Body_d730dae6

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "const": "project",
      "title": "Template Type",
      "description": "Saved templates support project content only.",
      "examples": [
        "project"
      ]
    },
    "name": {
      "type": "string",
      "maxLength": 255,
      "title": "Name",
      "description": "Saved template name, up to 255 characters.",
      "examples": [
        "Weekly planning"
      ]
    },
    "description": {
      "type": "string",
      "maxLength": 1000,
      "title": "Description",
      "description": "Saved template description, up to 1,000 characters.",
      "examples": [
        "Plan and review the week."
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "Source project. Required even with content, for access and provenance.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "color": {
      "$ref": "#/components/schemas/APIColorID",
      "description": "Template color.",
      "examples": [
        "berry_red"
      ]
    },
    "for_workspace": {
      "type": "boolean",
      "title": "For Workspace",
      "description": "Associate with the source project workspace as a restricted template. Defaults to a personal template.",
      "default": false,
      "examples": [
        false
      ]
    },
    "content": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/TemplateContent"
        },
        {
          "type": "null"
        }
      ],
      "title": "Content",
      "description": "Full replacement graph instead of exporting the source project. Uses the supported writable fields of a raw preview, including optional calendar defaults. IDs refer only to nodes in this payload. project_id is still required for access and provenance."
    }
  },
  "type": "object",
  "required": [
    "template_type",
    "name",
    "description",
    "project_id",
    "color"
  ],
  "title": "Body"
}
```

### Body_import_into_project_from_file_api_v1_templates_import_into_project_from_file_post

```json
{
  "properties": {
    "project_id": {
      "type": "string",
      "title": "Project Id"
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
    "project_id",
    "file"
  ],
  "title": "Body_import_into_project_from_file_api_v1_templates_import_into_project_from_file_post"
}
```

### Body_import_user_template_from_file_api_v1_templates_user_import_post

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "const": "project",
      "title": "Template Type",
      "description": "Saved templates support project content only.",
      "examples": [
        "project"
      ]
    },
    "name": {
      "type": "string",
      "maxLength": 255,
      "title": "Name",
      "description": "Saved template name, up to 255 characters.",
      "examples": [
        "Weekly planning"
      ]
    },
    "description": {
      "type": "string",
      "maxLength": 1000,
      "title": "Description",
      "description": "Saved template description, up to 1,000 characters.",
      "examples": [
        "Plan and review the week."
      ]
    },
    "color": {
      "$ref": "#/components/schemas/APIColorID",
      "description": "Template color.",
      "examples": [
        "berry_red"
      ]
    },
    "uploaded_file_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Uploaded File Name",
      "description": "Staged filename from /templates/user/preview, owned by the caller. Mutually exclusive with content.",
      "examples": [
        "1234567_abcdef_weekly-planning.csv"
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
    "template_type",
    "name",
    "description",
    "color"
  ],
  "title": "Body_import_user_template_from_file_api_v1_templates_user_import_post"
}
```

### Body_preview_user_template_api_v1_templates_user_preview_post

```json
{
  "properties": {
    "resolve_dates": {
      "type": "boolean",
      "title": "Resolve Dates",
      "description": "Resolve due dates and deadlines. Use false for an editor to receive raw date strings for the supported content graph projection. On v1.6+, raw typed dues also include date_type, time and timezone to preserve their meaning when saved.",
      "default": true,
      "examples": [
        false
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
    "file"
  ],
  "title": "Body_preview_user_template_api_v1_templates_user_preview_post"
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

### CategoriesResponse

```json
{
  "properties": {
    "categories": {
      "items": {
        "$ref": "#/components/schemas/ContentfulCategory"
      },
      "type": "array",
      "title": "Categories",
      "description": "Template categories in gallery display order."
    }
  },
  "type": "object",
  "required": [
    "categories"
  ],
  "title": "CategoriesResponse"
}
```

### ContentCalendarSettings

```json
{
  "properties": {
    "layout": {
      "$ref": "#/components/schemas/CalendarLayout",
      "description": "Calendar layout. This existing field uses uppercase WEEK or MONTH wire values.",
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
  "additionalProperties": false,
  "type": "object",
  "title": "ContentCalendarSettings",
  "description": "Calendar settings supplied as part of a template content graph."
}
```

### ContentDeadlineObject

```json
{
  "properties": {
    "date": {
      "type": "string",
      "pattern": "^\\d{4}-\\d{2}-\\d{2}$",
      "format": "date",
      "title": "Date",
      "description": "Absolute deadline in YYYY-MM-DD format.",
      "examples": [
        "2026-10-20"
      ]
    },
    "lang": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 10
        },
        {
          "type": "null"
        }
      ],
      "title": "Lang",
      "description": "Supported date language metadata.",
      "examples": [
        "en",
        "fr"
      ]
    }
  },
  "additionalProperties": false,
  "type": "object",
  "required": [
    "date"
  ],
  "title": "ContentDeadlineObject",
  "description": "An absolute deadline calendar date, without recurrence or time."
}
```

### ContentDueObject

```json
{
  "properties": {
    "date": {
      "type": "string",
      "maxLength": 64,
      "minLength": 1,
      "title": "Date",
      "description": "Authoritative ISO date or datetime. Template dues remain relative to the save day.",
      "examples": [
        "2026-10-20",
        "2026-10-20T10:00:00",
        "2026-10-20T08:00:00Z"
      ]
    },
    "lang": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 10
        },
        {
          "type": "null"
        }
      ],
      "title": "Lang",
      "description": "Supported date language metadata.",
      "examples": [
        "en",
        "fr"
      ]
    },
    "string": {
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
      "title": "String",
      "description": "Display text or recurrence rule. It never overrides date and must agree with is_recurring.",
      "examples": [
        "20 October",
        "tous les vendredis"
      ]
    },
    "timezone": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 64,
          "minLength": 1
        },
        {
          "type": "null"
        }
      ],
      "title": "Timezone",
      "description": "Timezone name required for an offset-bearing date. Validated but dropped for floating dates.",
      "examples": [
        "Europe/Paris"
      ]
    },
    "is_recurring": {
      "type": "boolean",
      "title": "Is Recurring",
      "description": "Whether string is a recurrence rule. Recurring dues require string.",
      "default": false,
      "examples": [
        false,
        true
      ]
    }
  },
  "additionalProperties": false,
  "type": "object",
  "required": [
    "date"
  ],
  "title": "ContentDueObject",
  "description": "A complete due with explicit timezone and recurrence intent."
}
```

### ContentDuration

```json
{
  "properties": {
    "amount": {
      "type": "integer",
      "exclusiveMinimum": 0.0,
      "title": "Amount",
      "description": "Positive duration amount.",
      "examples": [
        30
      ]
    },
    "unit": {
      "type": "string",
      "enum": [
        "minute",
        "day"
      ],
      "title": "Unit",
      "description": "Duration unit.",
      "examples": [
        "minute"
      ]
    }
  },
  "type": "object",
  "required": [
    "amount",
    "unit"
  ],
  "title": "ContentDuration"
}
```

### ContentFileAttachment

```json
{
  "properties": {
    "file_name": {
      "type": "string",
      "minLength": 1,
      "title": "File Name",
      "description": "Attachment filename.",
      "examples": [
        "agenda.pdf"
      ]
    },
    "file_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Url",
      "description": "Attachment URL, subject to the attachment URL allow-list.",
      "examples": [
        "https://files.todoist.com/abcdef/agenda.pdf"
      ]
    },
    "file_size": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Size",
      "description": "Attachment size in bytes.",
      "examples": [
        1024
      ]
    },
    "file_type": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Type",
      "description": "Attachment MIME type.",
      "examples": [
        "application/pdf"
      ]
    },
    "image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Image",
      "description": "Attachment image URL, subject to the attachment URL allow-list.",
      "examples": [
        "https://files.todoist.com/abcdef/image.png"
      ]
    },
    "upload_state": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Upload State",
      "description": "Attachment upload state.",
      "examples": [
        "completed"
      ]
    }
  },
  "type": "object",
  "required": [
    "file_name"
  ],
  "title": "ContentFileAttachment",
  "description": "The fields templates actually round-trip through `FileAttachment`. Typed\n(rather than a raw dict) so a malformed client value fails validation\nwith a 400 here instead of crashing string operations (e.g. `.replace()`\non `file_name`) later in CSV serialization. Unrecognized keys (e.g.\nserver-computed thumbnail fields the client is just echoing back from\npreview) are ignored rather than rejected.\n\n`file_name` is required: `append_file_attachment` only emits an\nattachment marker when a name is present, so a file_attachment without\none would validate here yet vanish silently at save time (and, absent\nany other content, take the whole comment down with it - see\n`ContentProjectNote._validate_has_content`)."
}
```

### ContentItem

```json
{
  "properties": {
    "id": {
      "type": "string",
      "maxLength": 64,
      "minLength": 1,
      "title": "Id",
      "description": "Opaque task reference, unique within this graph.",
      "examples": [
        "task-1"
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
      "description": "Parent task reference within this graph.",
      "examples": [
        null
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
      "description": "Section reference within this graph.",
      "examples": [
        "section-1"
      ]
    },
    "content": {
      "type": "string",
      "maxLength": 2048,
      "title": "Content",
      "description": "Task text. May be empty only when labels are present.",
      "default": "",
      "examples": [
        "Plan the week"
      ]
    },
    "description": {
      "type": "string",
      "maxLength": 16383,
      "title": "Description",
      "description": "Task description.",
      "default": "",
      "examples": [
        "Review upcoming work."
      ]
    },
    "priority": {
      "type": "integer",
      "maximum": 4.0,
      "minimum": 1.0,
      "title": "Priority",
      "description": "Sync priority: 1 is normal and 4 is urgent.",
      "default": 1,
      "examples": [
        4
      ]
    },
    "labels": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "maxItems": 100,
      "title": "Labels",
      "description": "Label names attached to the task.",
      "examples": [
        [
          "planning"
        ]
      ]
    },
    "due": {
      "anyOf": [
        {
          "oneOf": [
            {
              "$ref": "#/components/schemas/ContentDueObject"
            },
            {
              "$ref": "#/components/schemas/ContentRawDue"
            }
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Due",
      "description": "Resolved object or raw string/language (v1.6+). Omission or null clears the due in this replacement graph.",
      "examples": [
        {
          "date": "2026-10-20"
        },
        {
          "lang": "en",
          "string": "in 3 days"
        },
        null
      ]
    },
    "deadline": {
      "anyOf": [
        {
          "oneOf": [
            {
              "$ref": "#/components/schemas/ContentDeadlineObject"
            },
            {
              "$ref": "#/components/schemas/ContentRawDeadline"
            }
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Deadline",
      "description": "Absolute date object or raw string/language (v1.6+). Omission or null clears the deadline.",
      "examples": [
        {
          "date": "2026-10-20",
          "lang": "en"
        },
        {
          "lang": "en",
          "string": "2026-10-20"
        },
        null
      ]
    },
    "duration": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentDuration"
        },
        {
          "type": "null"
        }
      ],
      "description": "Task duration."
    },
    "responsible_uid": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^\\d+$"
        },
        {
          "type": "null"
        }
      ],
      "title": "Responsible Uid",
      "description": "Assignee user ID.",
      "examples": [
        "1234567"
      ]
    },
    "assigned_by_uid": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^\\d+$"
        },
        {
          "type": "null"
        }
      ],
      "title": "Assigned By Uid",
      "description": "Assigning user ID.",
      "examples": [
        "1234567"
      ]
    }
  },
  "type": "object",
  "required": [
    "id"
  ],
  "title": "ContentItem"
}
```

### ContentNote

```json
{
  "properties": {
    "content": {
      "type": "string",
      "maxLength": 15000,
      "title": "Content",
      "description": "Comment text. May be empty when an attachment is present.",
      "default": "",
      "examples": [
        "Planning notes."
      ]
    },
    "posted_uid": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^\\d+$"
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted Uid",
      "description": "Comment author user ID.",
      "examples": [
        "1234567"
      ]
    },
    "posted_at": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 64
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted At",
      "description": "Comment timestamp.",
      "examples": [
        "2026-10-16T10:00:00Z"
      ]
    },
    "file_attachment": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentFileAttachment"
        },
        {
          "type": "null"
        }
      ],
      "description": "Comment attachment. Unrecognized preview fields are ignored."
    },
    "item_id": {
      "type": "string",
      "maxLength": 64,
      "minLength": 1,
      "title": "Item Id",
      "description": "Task reference within this graph.",
      "examples": [
        "task-1"
      ]
    }
  },
  "type": "object",
  "required": [
    "item_id"
  ],
  "title": "ContentNote"
}
```

### ContentProject

```json
{
  "properties": {
    "view_style": {
      "$ref": "#/components/schemas/ProjectViewStyleV1",
      "description": "Project layout.",
      "default": "list",
      "examples": [
        "calendar"
      ]
    },
    "description": {
      "type": "string",
      "maxLength": 16383,
      "title": "Description",
      "description": "Project description.",
      "default": "",
      "examples": [
        "A weekly planning workflow."
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
      "description": "Project icon name, or null for the default icon.",
      "examples": [
        "briefcase"
      ]
    }
  },
  "type": "object",
  "title": "ContentProject"
}
```

### ContentProjectNote

```json
{
  "properties": {
    "content": {
      "type": "string",
      "maxLength": 15000,
      "title": "Content",
      "description": "Comment text. May be empty when an attachment is present.",
      "default": "",
      "examples": [
        "Planning notes."
      ]
    },
    "posted_uid": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^\\d+$"
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted Uid",
      "description": "Comment author user ID.",
      "examples": [
        "1234567"
      ]
    },
    "posted_at": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 64
        },
        {
          "type": "null"
        }
      ],
      "title": "Posted At",
      "description": "Comment timestamp.",
      "examples": [
        "2026-10-16T10:00:00Z"
      ]
    },
    "file_attachment": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentFileAttachment"
        },
        {
          "type": "null"
        }
      ],
      "description": "Comment attachment. Unrecognized preview fields are ignored."
    }
  },
  "type": "object",
  "title": "ContentProjectNote"
}
```

### ContentProjectViewOptionsDefaults

```json
{
  "properties": {
    "calendar_settings": {
      "$ref": "#/components/schemas/ContentCalendarSettings",
      "description": "Calendar defaults for the sole calendar project. No project ID is needed."
    }
  },
  "additionalProperties": false,
  "type": "object",
  "required": [
    "calendar_settings"
  ],
  "title": "ContentProjectViewOptionsDefaults",
  "description": "Calendar defaults implicitly associated with the template's sole project."
}
```

### ContentRawDeadline

```json
{
  "not": {
    "required": [
      "date"
    ]
  },
  "properties": {
    "string": {
      "type": "string",
      "maxLength": 255,
      "minLength": 1,
      "title": "String",
      "description": "Natural-language or ISO date. Nonrecurring task due dates are stored relative to the save day; deadlines retain their supplied strings.",
      "examples": [
        "in 3 days"
      ]
    },
    "lang": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 10
        },
        {
          "type": "null"
        }
      ],
      "title": "Lang",
      "description": "Language of the date expression.",
      "examples": [
        "en"
      ]
    }
  },
  "type": "object",
  "required": [
    "string"
  ],
  "title": "ContentRawDeadline",
  "description": "An unresolved deadline expression without an authoritative date."
}
```

### ContentRawDue

```json
{
  "not": {
    "required": [
      "date"
    ]
  },
  "properties": {
    "string": {
      "type": "string",
      "maxLength": 255,
      "minLength": 1,
      "title": "String",
      "description": "Natural-language or ISO date. Nonrecurring task due dates are stored relative to the save day; deadlines retain their supplied strings.",
      "examples": [
        "in 3 days"
      ]
    },
    "lang": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 10
        },
        {
          "type": "null"
        }
      ],
      "title": "Lang",
      "description": "Language of the date expression.",
      "examples": [
        "en"
      ]
    },
    "date_type": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "floating_date",
            "floating_datetime",
            "fixed_datetime"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Date Type",
      "description": "Explicit due type from an unresolved typed preview (v1.6+).",
      "examples": [
        "floating_datetime"
      ]
    },
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
      "description": "Independent local clock time. Required for timed date_type values.",
      "examples": [
        "10:00:17"
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
      "description": "Timezone name, required only for fixed_datetime. Existing supported aliases are accepted.",
      "examples": [
        "Europe/Paris"
      ]
    }
  },
  "type": "object",
  "required": [
    "string"
  ],
  "title": "ContentRawDue",
  "description": "An unresolved due, optionally carrying typed metadata from raw preview."
}
```

### ContentSection

```json
{
  "properties": {
    "id": {
      "type": "string",
      "maxLength": 64,
      "minLength": 1,
      "title": "Id",
      "description": "Opaque section reference, unique within this graph.",
      "examples": [
        "section-1"
      ]
    },
    "name": {
      "type": "string",
      "maxLength": 2048,
      "minLength": 1,
      "title": "Name",
      "description": "Section name.",
      "examples": [
        "This week"
      ]
    },
    "description": {
      "anyOf": [
        {
          "type": "string",
          "maxLength": 16383
        },
        {
          "type": "null"
        }
      ],
      "title": "Description",
      "description": "Section description.",
      "examples": [
        "Work planned for this week."
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name"
  ],
  "title": "ContentSection"
}
```

### ContentfulCategory

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "Category slug.",
      "examples": [
        "work"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Display name.",
      "examples": [
        "Work"
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
      "description": "Description. May be null when no description is available.",
      "examples": [
        "Track and prioritize bugs."
      ]
    },
    "seo": {
      "$ref": "#/components/schemas/ContentfulSeo",
      "description": "Search engine metadata for the category."
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "description",
    "seo"
  ],
  "title": "ContentfulCategory"
}
```

### ContentfulLink

```json
{
  "properties": {
    "title": {
      "type": "string",
      "title": "Title",
      "description": "Display title.",
      "examples": [
        "Bug tracking"
      ]
    },
    "url": {
      "type": "string",
      "title": "Url",
      "description": "Resource URL.",
      "examples": [
        "https://todoist.com/templates/bug-tracking"
      ]
    }
  },
  "type": "object",
  "required": [
    "title",
    "url"
  ],
  "title": "ContentfulLink"
}
```

### ContentfulSeo

```json
{
  "properties": {
    "title": {
      "type": "string",
      "title": "Title",
      "description": "Display title.",
      "examples": [
        "Bug tracking"
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
      "description": "Description. May be null when no description is available.",
      "examples": [
        "Track and prioritize bugs."
      ]
    }
  },
  "type": "object",
  "required": [
    "title",
    "description"
  ],
  "title": "ContentfulSeo"
}
```

### CreateFromTemplateIdResponse

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "enum": [
        "project",
        "setup"
      ],
      "title": "Template Type",
      "description": "Template type: project or setup.",
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
      "description": "Projects in the template's sync-style graph."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the template's sync-style graph."
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Items",
      "description": "Tasks in the graph. Uses the existing sync-style `items` name."
    },
    "notes": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Notes",
      "description": "Task comments in the graph. Uses the existing sync-style `notes` name."
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments in the graph."
    },
    "labels": {
      "items": {
        "$ref": "#/components/schemas/LabelSyncView"
      },
      "type": "array",
      "title": "Labels",
      "description": "Labels in a setup, or an empty list for project use."
    },
    "filters": {
      "items": {
        "$ref": "#/components/schemas/FilterSyncView"
      },
      "type": "array",
      "title": "Filters",
      "description": "Filters in a setup, or an empty list for project use."
    },
    "view_options": {
      "items": {
        "$ref": "#/components/schemas/UserViewOptionsSyncView"
      },
      "type": "array",
      "title": "View Options",
      "description": "User view options in the template graph."
    },
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Always ok on success.",
      "examples": [
        "ok"
      ]
    },
    "label_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Label Ids",
      "description": "Created label IDs.",
      "examples": [
        [
          "1234567"
        ]
      ]
    },
    "filter_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Filter Ids",
      "description": "Created filter IDs.",
      "examples": [
        [
          "1234568"
        ]
      ]
    },
    "project_ids": {
      "items": {
        "$ref": "#/components/schemas/ProjectResponse"
      },
      "type": "array",
      "title": "Project Ids",
      "description": "Created project IDs, each wrapped in a project_id object."
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
    }
  },
  "type": "object",
  "required": [
    "template_type",
    "projects",
    "sections",
    "items",
    "notes",
    "project_notes",
    "labels",
    "filters",
    "view_options",
    "status",
    "label_ids",
    "filter_ids",
    "project_ids",
    "reminders"
  ],
  "title": "CreateFromTemplateIdResponse"
}
```

### CreateUserTemplateResponse

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "Curated template slug or saved template ID (UT_ prefix).",
      "examples": [
        "bug-tracking",
        "UT_28Rc62VC22j439VcPJMw9xgWx2"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Display name.",
      "examples": [
        "Bug tracking"
      ]
    },
    "template_type": {
      "type": "string",
      "enum": [
        "setup",
        "project"
      ],
      "title": "Template Type",
      "description": "Project template or a complete setup.",
      "examples": [
        "project"
      ]
    },
    "short_description": {
      "type": "string",
      "title": "Short Description",
      "description": "Short template description.",
      "examples": [
        "Track and prioritize bugs."
      ]
    },
    "long_description": {
      "type": "string",
      "title": "Long Description",
      "description": "Full template description.",
      "examples": [
        "Track issues from report to resolution."
      ]
    },
    "instructions": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Instructions",
      "description": "Template instructions, or null.",
      "examples": [
        null
      ]
    },
    "import_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Import Url",
      "description": "Curated template import URL. Omitted for saved templates.",
      "examples": [
        "https://todoist.com/api/v1/import/project_from_url?t_url=https://example.com/template.csv"
      ]
    },
    "view_type": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectViewStyleV1"
        },
        {
          "type": "null"
        }
      ],
      "description": "Project view. Omitted for setup templates and nullable for curated projects.",
      "examples": [
        "board"
      ]
    },
    "thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image",
      "description": "Thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "thumbnail_image_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image Dark",
      "description": "Thumbnail image dark for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "cover_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Cover Image",
      "description": "Deprecated alias for thumbnail_image.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "preview_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Preview Image",
      "description": "Preview image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "template_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Template Color",
      "description": "Saved template color name. Null for curated templates.",
      "examples": [
        "magenta"
      ]
    },
    "background_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color",
      "description": "Background color for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "background_color_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color Dark",
      "description": "Background color dark for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "creator": {
      "$ref": "#/components/schemas/Person",
      "description": "Template creator. Deleted accounts use a placeholder person."
    },
    "seo": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentfulSeo"
        },
        {
          "type": "null"
        }
      ],
      "description": "Search engine metadata. Null for saved templates."
    },
    "more_resources": {
      "items": {
        "$ref": "#/components/schemas/ContentfulLink"
      },
      "type": "array",
      "title": "More Resources",
      "description": "Related links for curated templates."
    },
    "category_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Category Ids",
      "description": "Curated category slugs. Empty for saved templates.",
      "examples": [
        [
          "work"
        ]
      ]
    },
    "project_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Project Count",
      "description": "Number of projects in a curated setup, if known.",
      "examples": [
        3
      ]
    },
    "label_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Label Count",
      "description": "Number of labels in a curated setup, if known.",
      "examples": [
        2
      ]
    },
    "filter_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Filter Count",
      "description": "Number of filters in a curated setup, if known.",
      "examples": [
        1
      ]
    },
    "video_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Url",
      "description": "Video url for curated templates, or null.",
      "examples": [
        "https://example.com/video.mp4"
      ]
    },
    "video_preview_image_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Preview Image Url",
      "description": "Video preview image url for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "video_thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Thumbnail Image",
      "description": "Video thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
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
      "description": "Associated workspace ID, null for personal templates. Omitted for curated templates.",
      "examples": [
        "123456"
      ]
    },
    "shared_with_workspace_members": {
      "type": "boolean",
      "title": "Shared With Workspace Members",
      "description": "Whether workspace members may view the saved template. Retained legacy field name.",
      "examples": [
        false
      ]
    },
    "can_edit": {
      "type": "boolean",
      "title": "Can Edit",
      "description": "Whether the caller may edit this saved template.",
      "examples": [
        true
      ]
    },
    "last_edited_by": {
      "$ref": "#/components/schemas/Person",
      "description": "Last editor of a saved template. Personal templates return the creator."
    },
    "last_edited_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Last Edited At",
      "description": "Last workspace edit timestamp, or null if unknown or personal.",
      "examples": [
        "2026-10-16T10:00:00Z"
      ]
    },
    "template_source": {
      "type": "string",
      "enum": [
        "doist",
        "user"
      ],
      "title": "Template Source",
      "description": "Curated templates use doist. Personal and workspace templates use user.",
      "examples": [
        "doist"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "template_type",
    "short_description",
    "long_description",
    "instructions",
    "thumbnail_image",
    "thumbnail_image_dark",
    "cover_image",
    "preview_image",
    "template_color",
    "background_color",
    "background_color_dark",
    "creator",
    "seo",
    "more_resources",
    "category_ids",
    "template_source"
  ],
  "title": "CreateUserTemplateResponse"
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

### FileURLResponse

```json
{
  "properties": {
    "file_name": {
      "type": "string",
      "title": "File Name",
      "description": "Name of the exported template file.",
      "examples": [
        "Project.csv"
      ]
    },
    "file_url": {
      "type": "string",
      "format": "uri",
      "title": "File Url",
      "description": "Publicly accessible URL of the stored template CSV. No expiry is set.",
      "examples": [
        "https://example.com/Project.csv"
      ]
    }
  },
  "type": "object",
  "required": [
    "file_name",
    "file_url"
  ],
  "title": "FileURLResponse"
}
```

### FilterSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the filter.",
      "examples": [
        "4638878"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the filter.",
      "examples": [
        "Important"
      ]
    },
    "query": {
      "type": "string",
      "title": "Query",
      "description": "Query the filter searches for. See the [introduction to filters](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) for examples.",
      "examples": [
        "priority 1"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Description of the filter, as markdown. Empty when the filter has none.",
      "examples": [
        "Everything I need for the launch this week."
      ]
    },
    "color": {
      "type": "string",
      "title": "Color",
      "description": "Name of the filter icon color. Refer to the `name` column in the [Colors](https://developer.todoist.com/api/v1/colors.md) guide.",
      "examples": [
        "lime_green"
      ]
    },
    "item_order": {
      "type": "integer",
      "title": "Item Order",
      "description": "Position of the filter in the filter list; the smallest value sorts first.",
      "examples": [
        3
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
      "description": "Fractional-indexing order key; filters sort by comparing keys lexicographically. `null` for accounts not yet migrated.",
      "examples": [
        "a3"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the filter is marked as deleted.",
      "examples": [
        false
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the filter is a favorite.",
      "examples": [
        false
      ]
    },
    "is_frozen": {
      "type": "boolean",
      "title": "Is Frozen",
      "description": "Whether the filter is read-only because it belongs to a canceled subscription.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "query",
    "description",
    "color",
    "item_order",
    "order_key",
    "is_deleted",
    "is_favorite",
    "is_frozen"
  ],
  "title": "FilterSyncView",
  "description": "A class with fields representing a Filter which will be returned to clients in a\nsync (or sync-like) response."
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

### GenerateDescriptionResponse

```json
{
  "properties": {
    "description": {
      "type": "string",
      "title": "Description",
      "description": "AI-generated template description.",
      "examples": [
        "A weekly planning workflow."
      ]
    }
  },
  "type": "object",
  "required": [
    "description"
  ],
  "title": "GenerateDescriptionResponse"
}
```

### GroupOption

```json
{
  "type": "string",
  "enum": [
    "ASSIGNEE",
    "ADDED_DATE",
    "DUE_DATE",
    "LABEL",
    "PRIORITY",
    "PROJECT",
    "WORKSPACE",
    "DEADLINE"
  ],
  "title": "GroupOption",
  "x-enumDescriptions": {
    "ADDED_DATE": "Added Date",
    "ASSIGNEE": "Assignee",
    "DEADLINE": "Deadline",
    "DUE_DATE": "Due Date",
    "LABEL": "Label",
    "PRIORITY": "Priority",
    "PROJECT": "Project",
    "WORKSPACE": "Workspace"
  }
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

### LabelSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "ID of the label.",
      "examples": [
        "2156154810"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Name of the label.",
      "examples": [
        "errands"
      ]
    },
    "color": {
      "type": "string",
      "title": "Color",
      "description": "Color of the label icon, as a name from the [Colors](https://developer.todoist.com/api/v1/colors.md) guide.",
      "examples": [
        "charcoal"
      ]
    },
    "item_order": {
      "type": "integer",
      "title": "Item Order",
      "description": "Position in the label list: the smallest value goes at the top.",
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
      "description": "Fractional-indexing order key: labels sort by comparing keys lexicographically. May be `null` for accounts not yet migrated.",
      "examples": [
        "a0"
      ]
    },
    "is_favorite": {
      "type": "boolean",
      "title": "Is Favorite",
      "description": "Whether the label is a favorite.",
      "examples": [
        false
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the label is deleted.",
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
    "item_order",
    "order_key",
    "is_favorite",
    "is_deleted"
  ],
  "title": "LabelSyncView"
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

### PaginatedList_PublicTemplate_

```json
{
  "properties": {
    "results": {
      "items": {
        "$ref": "#/components/schemas/PublicTemplate"
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

### Person

```json
{
  "properties": {
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "Todoist user ID. Omitted for curated creators and deleted accounts.",
      "examples": [
        "123456"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Display name.",
      "examples": [
        "Alex Example"
      ]
    },
    "position": {
      "type": "string",
      "title": "Position",
      "description": "Creator position. Empty for Todoist users.",
      "examples": [
        ""
      ]
    },
    "avatar": {
      "type": "string",
      "title": "Avatar",
      "description": "Avatar image URL. Deleted accounts use a placeholder.",
      "examples": [
        "https://example.com/avatar.png"
      ]
    }
  },
  "type": "object",
  "required": [
    "name",
    "position",
    "avatar"
  ],
  "title": "Person"
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

### PreviewFromFileResponse

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "enum": [
        "project",
        "setup"
      ],
      "title": "Template Type",
      "description": "Template type: project or setup.",
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
      "description": "Projects in the template's sync-style graph."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the template's sync-style graph."
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Items",
      "description": "Tasks in the graph. Uses the existing sync-style `items` name."
    },
    "notes": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Notes",
      "description": "Task comments in the graph. Uses the existing sync-style `notes` name."
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments in the graph."
    },
    "project_view_options_defaults": {
      "items": {
        "$ref": "#/components/schemas/TemplateProjectViewOptionsDefaultsSnapshot"
      },
      "type": "array",
      "title": "Project View Options Defaults",
      "description": "Calendar settings stored by the project template, implicitly associated with its sole project.",
      "examples": [
        [
          {
            "calendar_settings": {
              "color": "PRIORITY",
              "layout": "WEEK",
              "visible_day_count": 3
            }
          }
        ]
      ]
    },
    "uploaded_file_name": {
      "type": "string",
      "title": "Uploaded File Name",
      "description": "Staged CSV filename owned by the caller. Pass it to /templates/user/import to save the template.",
      "examples": [
        "1234567_abcdef_weekly-planning.csv"
      ]
    },
    "reminders": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
      },
      "type": "array",
      "title": "Reminders",
      "description": "Always empty: templates don't carry reminders."
    }
  },
  "type": "object",
  "required": [
    "template_type",
    "projects",
    "sections",
    "items",
    "notes",
    "project_notes",
    "project_view_options_defaults",
    "uploaded_file_name",
    "reminders"
  ],
  "title": "PreviewFromFileResponse"
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

### ProjectResponse

```json
{
  "properties": {
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "ID of a created project.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    }
  },
  "type": "object",
  "required": [
    "project_id"
  ],
  "title": "ProjectResponse"
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

### ProjectTemplatePreviewResponse

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "enum": [
        "project",
        "setup"
      ],
      "title": "Template Type",
      "description": "Template type: project or setup.",
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
      "description": "Projects in the template's sync-style graph."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the template's sync-style graph."
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Items",
      "description": "Tasks in the graph. Uses the existing sync-style `items` name."
    },
    "notes": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Notes",
      "description": "Task comments in the graph. Uses the existing sync-style `notes` name."
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments in the graph."
    },
    "project_view_options_defaults": {
      "items": {
        "$ref": "#/components/schemas/TemplateProjectViewOptionsDefaultsSnapshot"
      },
      "type": "array",
      "title": "Project View Options Defaults",
      "description": "Calendar settings stored by the project template, implicitly associated with its sole project.",
      "examples": [
        [
          {
            "calendar_settings": {
              "color": "PRIORITY",
              "layout": "WEEK",
              "visible_day_count": 3
            }
          }
        ]
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
      "description": "Workspace associated with a saved template, otherwise null.",
      "examples": [
        null,
        "1234567"
      ]
    },
    "reminders": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
      },
      "type": "array",
      "title": "Reminders",
      "description": "Always empty: templates don't carry reminders."
    }
  },
  "type": "object",
  "required": [
    "template_type",
    "projects",
    "sections",
    "items",
    "notes",
    "project_notes",
    "project_view_options_defaults",
    "workspace_id",
    "reminders"
  ],
  "title": "ProjectTemplatePreviewResponse"
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

### PropertyAttributes

```json
{
  "properties": {
    "is_hidden": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Hidden",
      "description": "Whether the property is hidden.",
      "examples": [
        true
      ]
    },
    "width": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 4096.0,
          "minimum": 1.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Width",
      "description": "Width of the property's column, in CSS pixels.",
      "examples": [
        180
      ]
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "PropertyAttributes",
  "description": "How views show a property. Attributes the backend doesn't know are kept."
}
```

### PropertySettings

```json
{
  "properties": {
    "order": {
      "anyOf": [
        {
          "items": {
            "type": "string"
          },
          "type": "array",
          "maxItems": 256
        },
        {
          "type": "null"
        }
      ],
      "title": "Order",
      "description": "Property IDs in display order.",
      "examples": [
        [
          "due",
          "priority",
          "labels"
        ]
      ]
    },
    "properties": {
      "anyOf": [
        {
          "additionalProperties": {
            "anyOf": [
              {
                "$ref": "#/components/schemas/PropertyAttributes"
              },
              {
                "type": "null"
              }
            ]
          },
          "type": "object",
          "maxProperties": 256
        },
        {
          "type": "null"
        }
      ],
      "title": "Properties",
      "description": "How each property shows, by property ID.",
      "examples": [
        {
          "due": {
            "width": 180
          },
          "labels": {
            "is_hidden": true
          }
        }
      ]
    }
  },
  "additionalProperties": true,
  "type": "object",
  "title": "PropertySettings",
  "description": "How views show task properties, such as the due date or labels, holding only what the user\nchanged. Every layout reads the same settings: `width` applies where a layout shows the\nproperty in a column. Property IDs are opaque strings, so clients can add properties, like\ncustom fields, without a backend change, and keep the ones they don't know, as well as keys\nand attributes the backend doesn't know."
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

### PublicTemplate

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "Curated template slug or saved template ID (UT_ prefix).",
      "examples": [
        "bug-tracking",
        "UT_28Rc62VC22j439VcPJMw9xgWx2"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Display name.",
      "examples": [
        "Bug tracking"
      ]
    },
    "template_type": {
      "type": "string",
      "enum": [
        "setup",
        "project"
      ],
      "title": "Template Type",
      "description": "Project template or a complete setup.",
      "examples": [
        "project"
      ]
    },
    "short_description": {
      "type": "string",
      "title": "Short Description",
      "description": "Short template description.",
      "examples": [
        "Track and prioritize bugs."
      ]
    },
    "long_description": {
      "type": "string",
      "title": "Long Description",
      "description": "Full template description.",
      "examples": [
        "Track issues from report to resolution."
      ]
    },
    "instructions": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Instructions",
      "description": "Template instructions, or null.",
      "examples": [
        null
      ]
    },
    "import_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Import Url",
      "description": "Curated template import URL. Omitted for saved templates.",
      "examples": [
        "https://todoist.com/api/v1/import/project_from_url?t_url=https://example.com/template.csv"
      ]
    },
    "view_type": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectViewStyleV1"
        },
        {
          "type": "null"
        }
      ],
      "description": "Project view. Omitted for setup templates and nullable for curated projects.",
      "examples": [
        "board"
      ]
    },
    "thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image",
      "description": "Thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "thumbnail_image_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image Dark",
      "description": "Thumbnail image dark for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "cover_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Cover Image",
      "description": "Deprecated alias for thumbnail_image.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "preview_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Preview Image",
      "description": "Preview image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "template_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Template Color",
      "description": "Saved template color name. Null for curated templates.",
      "examples": [
        "magenta"
      ]
    },
    "background_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color",
      "description": "Background color for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "background_color_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color Dark",
      "description": "Background color dark for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "creator": {
      "$ref": "#/components/schemas/Person",
      "description": "Template creator. Deleted accounts use a placeholder person."
    },
    "seo": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentfulSeo"
        },
        {
          "type": "null"
        }
      ],
      "description": "Search engine metadata. Null for saved templates."
    },
    "more_resources": {
      "items": {
        "$ref": "#/components/schemas/ContentfulLink"
      },
      "type": "array",
      "title": "More Resources",
      "description": "Related links for curated templates."
    },
    "category_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Category Ids",
      "description": "Curated category slugs. Empty for saved templates.",
      "examples": [
        [
          "work"
        ]
      ]
    },
    "project_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Project Count",
      "description": "Number of projects in a curated setup, if known.",
      "examples": [
        3
      ]
    },
    "label_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Label Count",
      "description": "Number of labels in a curated setup, if known.",
      "examples": [
        2
      ]
    },
    "filter_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Filter Count",
      "description": "Number of filters in a curated setup, if known.",
      "examples": [
        1
      ]
    },
    "video_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Url",
      "description": "Video url for curated templates, or null.",
      "examples": [
        "https://example.com/video.mp4"
      ]
    },
    "video_preview_image_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Preview Image Url",
      "description": "Video preview image url for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "video_thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Thumbnail Image",
      "description": "Video thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
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
      "description": "Associated workspace ID, null for personal templates. Omitted for curated templates.",
      "examples": [
        "123456"
      ]
    },
    "shared_with_workspace_members": {
      "type": "boolean",
      "title": "Shared With Workspace Members",
      "description": "Whether workspace members may view the saved template. Retained legacy field name.",
      "examples": [
        false
      ]
    },
    "can_edit": {
      "type": "boolean",
      "title": "Can Edit",
      "description": "Whether the caller may edit this saved template.",
      "examples": [
        true
      ]
    },
    "last_edited_by": {
      "$ref": "#/components/schemas/Person",
      "description": "Last editor of a saved template. Personal templates return the creator."
    },
    "last_edited_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Last Edited At",
      "description": "Last workspace edit timestamp, or null if unknown or personal.",
      "examples": [
        "2026-10-16T10:00:00Z"
      ]
    },
    "template_source": {
      "type": "string",
      "enum": [
        "todoist",
        "user"
      ],
      "title": "Template Source",
      "description": "Curated templates use todoist. Personal and workspace templates use user.",
      "examples": [
        "todoist"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "template_type",
    "short_description",
    "long_description",
    "instructions",
    "thumbnail_image",
    "thumbnail_image_dark",
    "cover_image",
    "preview_image",
    "template_color",
    "background_color",
    "background_color_dark",
    "creator",
    "seo",
    "more_resources",
    "category_ids",
    "template_source"
  ],
  "title": "PublicTemplate"
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

### SetupTemplatePreviewResponse

```json
{
  "properties": {
    "template_type": {
      "type": "string",
      "enum": [
        "project",
        "setup"
      ],
      "title": "Template Type",
      "description": "Template type: project or setup.",
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
      "description": "Projects in the template's sync-style graph."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/SectionSyncView"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the template's sync-style graph."
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ItemSyncView"
      },
      "type": "array",
      "title": "Items",
      "description": "Tasks in the graph. Uses the existing sync-style `items` name."
    },
    "notes": {
      "items": {
        "$ref": "#/components/schemas/ItemNoteSyncView"
      },
      "type": "array",
      "title": "Notes",
      "description": "Task comments in the graph. Uses the existing sync-style `notes` name."
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ProjectNoteSyncView"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments in the graph."
    },
    "labels": {
      "items": {
        "$ref": "#/components/schemas/LabelSyncView"
      },
      "type": "array",
      "title": "Labels",
      "description": "Labels in a setup, or an empty list for project use."
    },
    "filters": {
      "items": {
        "$ref": "#/components/schemas/FilterSyncView"
      },
      "type": "array",
      "title": "Filters",
      "description": "Filters in a setup, or an empty list for project use."
    },
    "view_options": {
      "items": {
        "$ref": "#/components/schemas/UserViewOptionsSyncView"
      },
      "type": "array",
      "title": "View Options",
      "description": "User view options in the template graph."
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
      "description": "Workspace associated with a saved template, otherwise null.",
      "examples": [
        null,
        "1234567"
      ]
    },
    "folders": {
      "items": {
        "$ref": "#/components/schemas/FolderView"
      },
      "type": "array",
      "title": "Folders",
      "description": "Folders in the setup preview."
    },
    "reminders": {
      "items": {
        "$ref": "#/components/schemas/NotificationSyncView"
      },
      "type": "array",
      "title": "Reminders",
      "description": "Always empty: templates don't carry reminders."
    }
  },
  "type": "object",
  "required": [
    "template_type",
    "projects",
    "sections",
    "items",
    "notes",
    "project_notes",
    "labels",
    "filters",
    "view_options",
    "workspace_id",
    "folders",
    "reminders"
  ],
  "title": "SetupTemplatePreviewResponse"
}
```

### SortOption

```json
{
  "type": "string",
  "enum": [
    "ALPHABETICALLY",
    "ASSIGNEE",
    "ADDED_DATE",
    "DUE_DATE",
    "LABEL",
    "PRIORITY",
    "PROJECT",
    "WORKSPACE",
    "MANUAL",
    "DEADLINE"
  ],
  "title": "SortOption",
  "x-enumDescriptions": {
    "ADDED_DATE": "Added Date",
    "ALPHABETICALLY": "Alphabetically",
    "ASSIGNEE": "Assignee",
    "DEADLINE": "Deadline",
    "DUE_DATE": "Due Date",
    "LABEL": "Label",
    "MANUAL": "Manual",
    "PRIORITY": "Priority",
    "PROJECT": "Project",
    "WORKSPACE": "Workspace"
  }
}
```

### SortOrder

```json
{
  "type": "string",
  "enum": [
    "ASC",
    "DESC"
  ],
  "title": "SortOrder",
  "x-enumDescriptions": {
    "ASC": "Asc",
    "DESC": "Desc"
  }
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

### TemplateCalendarSettings

```json
{
  "properties": {
    "layout": {
      "$ref": "#/components/schemas/CalendarLayout",
      "default": "WEEK"
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
  "title": "TemplateCalendarSettings",
  "description": "Calendar settings representable by a persisted project template."
}
```

### TemplateContent

```json
{
  "properties": {
    "projects": {
      "items": {
        "$ref": "#/components/schemas/ContentProject"
      },
      "type": "array",
      "title": "Projects",
      "description": "Zero or one project, describing layout, description and icon.",
      "default": []
    },
    "project_view_options_defaults": {
      "items": {
        "$ref": "#/components/schemas/ContentProjectViewOptionsDefaults"
      },
      "type": "array",
      "maxItems": 1,
      "title": "Project View Options Defaults",
      "description": "Calendar settings implicitly associated with the sole calendar project. Objects do not include a project identifier. Invalid optional calendar metadata is ignored."
    },
    "sections": {
      "items": {
        "$ref": "#/components/schemas/ContentSection"
      },
      "type": "array",
      "title": "Sections",
      "description": "Sections in the replacement graph.",
      "default": []
    },
    "items": {
      "items": {
        "$ref": "#/components/schemas/ContentItem"
      },
      "type": "array",
      "title": "Items",
      "description": "Tasks in the replacement graph.",
      "default": []
    },
    "notes": {
      "items": {
        "$ref": "#/components/schemas/ContentNote"
      },
      "type": "array",
      "title": "Notes",
      "description": "Task comments, linked by item_id.",
      "default": []
    },
    "project_notes": {
      "items": {
        "$ref": "#/components/schemas/ContentProjectNote"
      },
      "type": "array",
      "title": "Project Notes",
      "description": "Project comments.",
      "default": []
    }
  },
  "type": "object",
  "title": "TemplateContent",
  "description": "Full replacement graph using the supported writable projection of a raw preview.\nIDs refer only to nodes within this payload. At most 5,000 objects across sections,\ntasks, comments and calendar defaults are allowed, with at least one section or\ntask. Setup templates are not supported. Templates don't carry reminders, so a\n`reminders` key is ignored like any other unknown one.\n\nCollection names (`items`, `notes`) and actor fields retain the existing Sync API names."
}
```

### TemplatePreviewResponse

```json
{
  "anyOf": [
    {
      "$ref": "#/components/schemas/ProjectTemplatePreviewResponse"
    },
    {
      "$ref": "#/components/schemas/SetupTemplatePreviewResponse"
    }
  ]
}
```

### TemplateProjectViewOptionsDefaultsSnapshot

```json
{
  "properties": {
    "calendar_settings": {
      "$ref": "#/components/schemas/TemplateCalendarSettings"
    }
  },
  "type": "object",
  "required": [
    "calendar_settings"
  ],
  "title": "TemplateProjectViewOptionsDefaultsSnapshot",
  "description": "Project-view defaults stored with a project template, without a project id."
}
```

### UpdateUserTemplateResponse

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "Curated template slug or saved template ID (UT_ prefix).",
      "examples": [
        "bug-tracking",
        "UT_28Rc62VC22j439VcPJMw9xgWx2"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Display name.",
      "examples": [
        "Bug tracking"
      ]
    },
    "template_type": {
      "type": "string",
      "enum": [
        "setup",
        "project"
      ],
      "title": "Template Type",
      "description": "Project template or a complete setup.",
      "examples": [
        "project"
      ]
    },
    "short_description": {
      "type": "string",
      "title": "Short Description",
      "description": "Short template description.",
      "examples": [
        "Track and prioritize bugs."
      ]
    },
    "long_description": {
      "type": "string",
      "title": "Long Description",
      "description": "Full template description.",
      "examples": [
        "Track issues from report to resolution."
      ]
    },
    "instructions": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Instructions",
      "description": "Template instructions, or null.",
      "examples": [
        null
      ]
    },
    "import_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Import Url",
      "description": "Curated template import URL. Omitted for saved templates.",
      "examples": [
        "https://todoist.com/api/v1/import/project_from_url?t_url=https://example.com/template.csv"
      ]
    },
    "view_type": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ProjectViewStyleV1"
        },
        {
          "type": "null"
        }
      ],
      "description": "Project view. Omitted for setup templates and nullable for curated projects.",
      "examples": [
        "board"
      ]
    },
    "thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image",
      "description": "Thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "thumbnail_image_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Thumbnail Image Dark",
      "description": "Thumbnail image dark for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "cover_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Cover Image",
      "description": "Deprecated alias for thumbnail_image.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "preview_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Preview Image",
      "description": "Preview image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "template_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Template Color",
      "description": "Saved template color name. Null for curated templates.",
      "examples": [
        "magenta"
      ]
    },
    "background_color": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color",
      "description": "Background color for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "background_color_dark": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Background Color Dark",
      "description": "Background color dark for curated templates, or null.",
      "examples": [
        "#ffffff"
      ]
    },
    "creator": {
      "$ref": "#/components/schemas/Person",
      "description": "Template creator. Deleted accounts use a placeholder person."
    },
    "seo": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/ContentfulSeo"
        },
        {
          "type": "null"
        }
      ],
      "description": "Search engine metadata. Null for saved templates."
    },
    "more_resources": {
      "items": {
        "$ref": "#/components/schemas/ContentfulLink"
      },
      "type": "array",
      "title": "More Resources",
      "description": "Related links for curated templates."
    },
    "category_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Category Ids",
      "description": "Curated category slugs. Empty for saved templates.",
      "examples": [
        [
          "work"
        ]
      ]
    },
    "project_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Project Count",
      "description": "Number of projects in a curated setup, if known.",
      "examples": [
        3
      ]
    },
    "label_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Label Count",
      "description": "Number of labels in a curated setup, if known.",
      "examples": [
        2
      ]
    },
    "filter_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Filter Count",
      "description": "Number of filters in a curated setup, if known.",
      "examples": [
        1
      ]
    },
    "video_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Url",
      "description": "Video url for curated templates, or null.",
      "examples": [
        "https://example.com/video.mp4"
      ]
    },
    "video_preview_image_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Preview Image Url",
      "description": "Video preview image url for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
      ]
    },
    "video_thumbnail_image": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Video Thumbnail Image",
      "description": "Video thumbnail image for curated templates, or null.",
      "examples": [
        "https://example.com/image.png"
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
      "description": "Associated workspace ID, null for personal templates. Omitted for curated templates.",
      "examples": [
        "123456"
      ]
    },
    "shared_with_workspace_members": {
      "type": "boolean",
      "title": "Shared With Workspace Members",
      "description": "Whether workspace members may view the saved template. Retained legacy field name.",
      "examples": [
        false
      ]
    },
    "can_edit": {
      "type": "boolean",
      "title": "Can Edit",
      "description": "Whether the caller may edit this saved template.",
      "examples": [
        true
      ]
    },
    "last_edited_by": {
      "$ref": "#/components/schemas/Person",
      "description": "Last editor of a saved template. Personal templates return the creator."
    },
    "last_edited_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "format": "date-time",
      "title": "Last Edited At",
      "description": "Last workspace edit timestamp, or null if unknown or personal.",
      "examples": [
        "2026-10-16T10:00:00Z"
      ]
    },
    "template_source": {
      "type": "string",
      "enum": [
        "doist",
        "user"
      ],
      "title": "Template Source",
      "description": "Curated templates use doist. Personal and workspace templates use user.",
      "examples": [
        "doist"
      ]
    },
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Update result.",
      "examples": [
        "ok"
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "template_type",
    "short_description",
    "long_description",
    "instructions",
    "thumbnail_image",
    "thumbnail_image_dark",
    "cover_image",
    "preview_image",
    "template_color",
    "background_color",
    "background_color_dark",
    "creator",
    "seo",
    "more_resources",
    "category_ids",
    "template_source",
    "status"
  ],
  "title": "UpdateUserTemplateResponse"
}
```

### UserViewOptionsSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the view options.",
      "examples": [
        "123456"
      ]
    },
    "view_type": {
      "$ref": "#/components/schemas/ViewType",
      "description": "The type of view these options apply to.",
      "examples": [
        "PROJECT"
      ]
    },
    "object_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Object Id",
      "description": "String ID of the object the options apply to (a project, label, or filter ID, depending on `view_type`), or `null` for the TODAY, UPCOMING, and ASSIGNED_TO_ME views.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "filtered_by": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Filtered By",
      "description": "Filter query applied to the view.",
      "examples": [
        "@urgent"
      ]
    },
    "grouped_by": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/GroupOption"
        },
        {
          "type": "null"
        }
      ],
      "description": "Attribute tasks are grouped by.",
      "examples": [
        "PRIORITY"
      ]
    },
    "sorted_by": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/SortOption"
        },
        {
          "type": "null"
        }
      ],
      "description": "Attribute tasks are sorted by.",
      "examples": [
        "DUE_DATE"
      ]
    },
    "sort_order": {
      "$ref": "#/components/schemas/SortOrder",
      "description": "Sort direction.",
      "examples": [
        "ASC"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "const": false,
      "title": "Is Deleted",
      "description": "Whether the view options are deleted.",
      "default": false,
      "examples": [
        false
      ]
    },
    "view_mode": {
      "$ref": "#/components/schemas/ViewMode",
      "description": "Layout used to display tasks.",
      "examples": [
        "LIST"
      ]
    },
    "show_completed_tasks": {
      "type": "boolean",
      "title": "Show Completed Tasks",
      "description": "Whether completed tasks are shown in the view.",
      "examples": [
        false
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
      "description": "Settings for the calendar view mode."
    },
    "property_settings": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PropertySettings"
        },
        {
          "type": "null"
        }
      ],
      "description": "How views show task properties, such as the due date, in every layout."
    }
  },
  "type": "object",
  "required": [
    "id",
    "view_type",
    "object_id",
    "filtered_by",
    "grouped_by",
    "sorted_by",
    "sort_order",
    "view_mode",
    "show_completed_tasks",
    "calendar_settings",
    "property_settings"
  ],
  "title": "UserViewOptionsSyncView",
  "description": "User-specific display settings for a task view."
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

### ViewType

```json
{
  "type": "string",
  "enum": [
    "FILTER",
    "LABEL",
    "PROJECT",
    "TODAY",
    "UPCOMING",
    "WORKSPACE_FILTER",
    "ASSIGNED_TO_ME"
  ],
  "title": "ViewType",
  "x-enumDescriptions": {
    "ASSIGNED_TO_ME": "Assigned To Me",
    "FILTER": "Filter",
    "LABEL": "Label",
    "PROJECT": "Project",
    "TODAY": "Today",
    "UPCOMING": "Upcoming",
    "WORKSPACE_FILTER": "Workspace Filter"
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
