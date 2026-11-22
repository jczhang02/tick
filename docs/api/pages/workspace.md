# Workspace

## Endpoints

### Delete Invitation

`POST /api/v1/workspaces/invitations/delete`

Deletes a workspace invitation. Only admins can delete invitations.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_52bab197"
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
            "$ref": "#/components/schemas/WorkspaceInvitationView"
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

### All Invitations

`GET /api/v1/workspaces/invitations/all`

Return a list containing details of all pending invitation to a workspace.

This list is not paginated. All workspace members can access this list.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the workspace.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "type": "array",
            "items": {
              "$ref": "#/components/schemas/WorkspaceInvitationView"
            },
            "title": "Response All Invitations Api V1 Workspaces Invitations All Get"
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

### Accept Invitation

`PUT /api/v1/workspaces/invitations/{invite_code}/accept`

Accept a workspace invitation. Usable by authenticated users only.

```json
{
  "parameters": [
    {
      "name": "invite_code",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "An opaque string representing an invite code. This invitation code is sent to a user via email and is exclusive for the user.",
        "examples": [
          "26d332457f16dc67f18385194be148f62c2b4317"
        ],
        "title": "Invite Code"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/WorkspaceInvitationView"
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

### Reject Invitation

`PUT /api/v1/workspaces/invitations/{invite_code}/reject`

Reject a workspace invitation. Usable by authenticated users only.

```json
{
  "parameters": [
    {
      "name": "invite_code",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "An opaque string representing an invite code. This invitation code is sent to a user via email and is exclusive for the user.",
        "examples": [
          "26d332457f16dc67f18385194be148f62c2b4317"
        ],
        "title": "Invite Code"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/WorkspaceInvitationView"
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

### Archived Projects

`GET /api/v1/workspaces/{workspace_id}/projects/archived`

Return archived projects in a workspace.

Workspace guests cannot list archived projects and receive a `FORBIDDEN`
error. Results are paginated by `cursor` and `limit`.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "minimum": 0,
        "description": "ID of the workspace whose archived projects should be listed.",
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
        "maximum": 500,
        "minimum": 1,
        "description": "The number of archived workspace projects to return in a page.",
        "examples": [
          50
        ],
        "default": 100,
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
            "$ref": "#/components/schemas/_ProjectsResponse"
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

### Active Projects

`GET /api/v1/workspaces/{workspace_id}/projects/active`

Returns all active workspace projects, including those visible but not joined by the user.

_For guests, returns all joined workspace projects only._

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "minimum": 0,
        "description": "ID of the workspace whose active projects should be listed.",
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
        "maximum": 500,
        "minimum": 1,
        "description": "The number of active workspace projects to return in a page.",
        "examples": [
          50
        ],
        "default": 100,
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
            "$ref": "#/components/schemas/_ProjectsResponse"
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

### Invite Workspace Users

`POST /api/v1/workspaces/{workspace_id}/users/invite`

Invites users to a workspace by email.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_108b7f01"
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
            "$ref": "#/components/schemas/InviteWorkspaceUsersResponse"
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

### Plan Details

`GET /api/v1/workspaces/plan_details`

Lists details of the workspace's current plan and usage

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the workspace.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_PlanDetailsResponse"
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

### Invitations

`GET /api/v1/workspaces/invitations`

Return a list of user emails who have a pending invitation to a workspace.

The list is not paginated. All workspace members can access this list.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "query",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "ID of the workspace.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "type": "array",
            "items": {
              "type": "string",
              "description": "Email of the invitation.",
              "examples": [
                "example@email.org"
              ]
            },
            "title": "Response Invitations Api V1 Workspaces Invitations Get"
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

### Get Workspaces Users

`GET /api/v1/workspaces/users`

Returns all workspace_users for a given workspace if workspace_id is
provided. Otherwise, returns all workspace_users for all workspaces that
the requesting user is part of.

_Not accessible by guests._

```json
{
  "parameters": [
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
        "description": "ID of the workspace to list users for.",
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
        "description": "Maximum number of workspace users to return.",
        "examples": [
          50
        ],
        "default": 100,
        "title": "Limit"
      }
    },
    {
      "name": "sort",
      "in": "query",
      "required": false,
      "schema": {
        "enum": [
          "id",
          "name"
        ],
        "type": "string",
        "description": "Order workspace users by ID, or by name with the requesting user first. Name sorting requires workspace_id.",
        "examples": [
          "name"
        ],
        "default": "id",
        "title": "Sort"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_UsersResponse"
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

### Join

`POST /api/v1/workspaces/join`

Join a workspace via link or via workspace ID, if the user can auto-join
the workspace by domain.

#### Joining by Domain
This is possible if:
- The user is verified
- The user has a user e-mail belonging to a domain that is set
  as a domain name for a workspace
- That workspace has the auto-join by domain feature enabled

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_fe5aa153"
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
            "$ref": "#/components/schemas/WorkspaceUserView"
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

### Update Logo

`POST /api/v1/workspaces/logo`

Upload an image to be used as the workspace logo. Similar to a user’s
avatar. If `delete` is set to true, it removes the logo completely and does
not return any `logo_*` attribute.

```json
{
  "requestBody": {
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_update_logo_api_v1_workspaces_logo_post"
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
            "anyOf": [
              {
                "$ref": "#/components/schemas/WorkspaceSyncView"
              },
              {
                "type": "null"
              }
            ],
            "title": "Response Update Logo Api V1 Workspaces Logo Post"
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

### Remove Workspace User

`DELETE /api/v1/workspaces/{workspace_id}/users/{user_id}`

Removes a user from a workspace.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    },
    {
      "name": "user_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMaximum": 18446744073709551616,
        "exclusiveMinimum": 0,
        "description": "ID of the workspace user",
        "examples": [
          123456
        ],
        "title": "User Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/RemoveWorkspaceUserResponse"
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

### Update Workspace User

`POST /api/v1/workspaces/{workspace_id}/users/{user_id}`

Updates a workspace user's role.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    },
    {
      "name": "user_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMaximum": 18446744073709551616,
        "exclusiveMinimum": 0,
        "description": "ID of the workspace user",
        "examples": [
          123456
        ],
        "title": "User Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_51341cd2"
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
            "$ref": "#/components/schemas/WorkspaceUserView"
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

### Delete Workspace

`DELETE /api/v1/workspaces/{workspace_id}`

Deletes a workspace for all of its members.

Only workspace admins may delete it; other members get `403`. Answers
`204` on success, and also when the workspace does not exist or was
already deleted, so retrying is safe. Requires the `data:read_write` scope.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
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

### Get Workspace

`GET /api/v1/workspaces/{workspace_id}`

Returns a workspace by ID.

The caller must be a member of the workspace. A workspace whose trial is
still being set up is returned too. Accepts tokens with the
`workspaces:read` scope (implied by `data:read`) from any Doist product.

Errors: `403` when the caller is not a member, `404` when the workspace
does not exist.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/WorkspaceSyncView"
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

### Update Workspace

`POST /api/v1/workspaces/{workspace_id}`

Updates an existing workspace and returns it.

```json
{
  "parameters": [
    {
      "name": "workspace_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "exclusiveMinimum": 0,
        "description": "ID of the workspace",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "requestBody": {
    "required": true,
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_b628cfc1"
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
            "$ref": "#/components/schemas/WorkspaceSyncView"
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

### Get Workspaces

`GET /api/v1/workspaces`

Returns all workspaces where the user is a member.

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "items": {
              "$ref": "#/components/schemas/WorkspaceSyncView"
            },
            "type": "array",
            "title": "Response Get Workspaces Api V1 Workspaces Get"
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

### Create Workspace

`POST /api/v1/workspaces`

Creates a new workspace and returns it.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_e1a528d7"
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
            "$ref": "#/components/schemas/WorkspaceSyncView"
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

### AcquisitionSource

```json
{
  "type": "string",
  "const": "high_paid_channel"
}
```

### Body_108b7f01

```json
{
  "properties": {
    "email_list": {
      "anyOf": [
        {
          "items": {
            "type": "string",
            "format": "email"
          },
          "type": "array"
        },
        {
          "type": "null"
        }
      ],
      "title": "Email List",
      "description": "List of user emails to invite to the workspace.",
      "examples": [
        [
          "teammate@example.com"
        ]
      ]
    },
    "role": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/WorkspaceRole"
        },
        {
          "type": "null"
        }
      ],
      "description": "Role assigned to invited users.",
      "examples": [
        "MEMBER"
      ]
    },
    "app": {
      "$ref": "#/components/schemas/ProductApp",
      "description": "For first-party clients usage only. This attribute may be removed or changed without notice, so we strongly advise not to rely on it. Product the invitation is sent from. Products without their own invite emails are recorded as Todoist.",
      "default": "todoist",
      "examples": [
        "comms"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_51341cd2

```json
{
  "properties": {
    "role": {
      "$ref": "#/components/schemas/WorkspaceRole",
      "description": "Updated role for the user.",
      "examples": [
        "ADMIN"
      ]
    }
  },
  "type": "object",
  "required": [
    "role"
  ],
  "title": "Body"
}
```

### Body_52bab197

```json
{
  "properties": {
    "workspace_id": {
      "type": "integer",
      "title": "Workspace Id",
      "description": "ID of the workspace containing the invitation.",
      "examples": [
        123456
      ]
    },
    "user_email": {
      "type": "string",
      "title": "User Email",
      "description": "Email address of the invited user.",
      "examples": [
        "teammate@example.com"
      ]
    }
  },
  "type": "object",
  "required": [
    "workspace_id",
    "user_email"
  ],
  "title": "Body"
}
```

### Body_b628cfc1

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
      "description": "Updated workspace name. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Design Team"
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
      "description": "Updated workspace description. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "Workspace for design projects."
      ]
    },
    "is_link_sharing_enabled": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Link Sharing Enabled",
      "description": "Updated link sharing status. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        true
      ]
    },
    "is_guest_allowed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Guest Allowed",
      "description": "Updated guest access status. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        true
      ]
    },
    "domain_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Domain Name",
      "description": "Updated workspace email domain. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "example.com"
      ]
    },
    "domain_discovery": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Domain Discovery",
      "description": "Updated domain discovery setting. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        false
      ]
    },
    "restrict_email_domains": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Restrict Email Domains",
      "description": "Updated email domain restriction setting. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        false
      ]
    },
    "properties": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PublicWorkspaceProperties"
        },
        {
          "type": "null"
        }
      ],
      "description": "Updated workspace properties. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        {}
      ]
    },
    "is_trial_pending": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Trial Pending",
      "description": "Updated trial pending state. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        false
      ]
    },
    "invite_code": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Invite Code",
      "description": "Regenerate workspace invite code when set. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        "regenerate"
      ]
    },
    "is_collapsed": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Collapsed",
      "description": "Updated collapse state for current user. Passing null or omitting this field will leave it unchanged.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_e1a528d7

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
      "description": "Name of the workspace.",
      "examples": [
        "Design Team"
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
      "description": "Description of the workspace.",
      "examples": [
        "Workspace for design projects."
      ]
    },
    "is_link_sharing_enabled": {
      "type": "boolean",
      "title": "Is Link Sharing Enabled",
      "description": "Whether link sharing is enabled for the workspace.",
      "default": true,
      "examples": [
        true
      ]
    },
    "is_guest_allowed": {
      "type": "boolean",
      "title": "Is Guest Allowed",
      "description": "Whether guests are allowed in the workspace.",
      "default": true,
      "examples": [
        true
      ]
    },
    "domain_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Domain Name",
      "description": "Workspace email domain.",
      "examples": [
        "example.com"
      ]
    },
    "domain_discovery": {
      "type": "boolean",
      "title": "Domain Discovery",
      "description": "Whether users with matching email domains can discover this workspace.",
      "default": false,
      "examples": [
        false
      ]
    },
    "restrict_email_domains": {
      "type": "boolean",
      "title": "Restrict Email Domains",
      "description": "Whether workspace invites are restricted to workspace domain.",
      "default": false,
      "examples": [
        false
      ]
    },
    "properties": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PublicWorkspaceProperties"
        },
        {
          "type": "null"
        }
      ],
      "description": "Workspace properties.",
      "examples": [
        {}
      ]
    },
    "is_trial_pending": {
      "type": "boolean",
      "title": "Is Trial Pending",
      "description": "Whether the workspace is waiting for trial confirmation.",
      "default": false,
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_fe5aa153

```json
{
  "properties": {
    "invite_code": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Invite Code",
      "description": "Workspace invite code to join with. Provide either `invite_code` or `workspace_id`, not both.",
      "examples": [
        "26d332457f16dc67f18385194be148f62c2b4317"
      ]
    },
    "workspace_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Workspace Id",
      "description": "Workspace ID to join by verified email domain. Provide either `invite_code` or `workspace_id`, not both.",
      "examples": [
        123456
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### Body_update_logo_api_v1_workspaces_logo_post

```json
{
  "properties": {
    "workspace_id": {
      "type": "integer",
      "minimum": 0.0,
      "title": "Workspace Id",
      "description": "ID of the workspace.",
      "examples": [
        123456
      ]
    },
    "delete": {
      "type": "boolean",
      "title": "Delete",
      "description": "Remove the current logo instead of uploading one.",
      "default": false,
      "examples": [
        false
      ]
    },
    "file": {
      "type": "string",
      "contentMediaType": "application/octet-stream",
      "title": "File",
      "description": "Image to use as the workspace logo, up to 4 MiB."
    }
  },
  "type": "object",
  "required": [
    "workspace_id",
    "file"
  ],
  "title": "Body_update_logo_api_v1_workspaces_logo_post"
}
```

### CollaboratorRole

```json
{
  "type": "string",
  "enum": [
    "CREATOR",
    "ADMIN",
    "READ_WRITE",
    "EDIT_ONLY",
    "COMPLETE_ONLY"
  ],
  "title": "CollaboratorRole",
  "description": "User role in the project. For personal project the role should be always \"CREATOR\"\nUser role for projects v1 maybe specified as \"CREATOR\" or \"ADMIN\", because in the\npast there was no permissions.",
  "x-enumDescriptions": {
    "ADMIN": "Admin",
    "COMPLETE_ONLY": "Complete Only",
    "CREATOR": "Creator",
    "EDIT_ONLY": "Edit Only",
    "READ_WRITE": "Read Write"
  }
}
```

### CreatedPlatform

```json
{
  "type": "string",
  "enum": [
    "web",
    "ios",
    "android"
  ]
}
```

### CreatorRole

```json
{
  "type": "string",
  "enum": [
    "owner_founder",
    "leader",
    "individual_contributor"
  ]
}
```

### DefaultAccessLevel

```json
{
  "type": "string",
  "enum": [
    "restricted",
    "team"
  ]
}
```

### DefaultCollaboratorsSyncView

```json
{
  "properties": {
    "user_ids": {
      "items": {
        "type": "integer"
      },
      "type": "array",
      "title": "User Ids",
      "description": "IDs of the members added to every new project in the workspace.",
      "examples": [
        [
          2671355
        ]
      ]
    },
    "predefined_group_ids": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Predefined Group Ids",
      "description": "Names of the predefined groups added to every new project in the workspace.",
      "examples": [
        [
          "MEMBERS_IN_WORKSPACE"
        ]
      ]
    }
  },
  "type": "object",
  "title": "DefaultCollaboratorsSyncView",
  "description": "Client representation of default collaborators within a workspace.\nSplit into 2 lists rather than using the polymorphic original\nWorkspaceDefaultCollaboratorView."
}
```

### Department

```json
{
  "type": "string",
  "enum": [
    "administration",
    "customer_service",
    "finance_accounting",
    "human_resources",
    "information_technology",
    "legal",
    "marketing",
    "operations",
    "product_development",
    "research_development",
    "sales",
    "supply_chain_management",
    "engineering",
    "quality_assurance",
    "executive_management",
    "other"
  ]
}
```

### DesktopWorkspaceModal

```json
{
  "type": "string",
  "enum": [
    "TRIAL_OFFER"
  ],
  "title": "DesktopWorkspaceModal",
  "description": "Enum to indicate when desktop should show workspace modal",
  "x-enumDescriptions": {
    "TRIAL_OFFER": "Trial Offer"
  }
}
```

### FormattedPrice

```json
{
  "properties": {
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO 4217 currency code.",
      "examples": [
        "USD"
      ]
    },
    "unit_amount": {
      "type": "integer",
      "title": "Unit Amount",
      "description": "Price amount in the currency's smallest unit.",
      "examples": [
        600
      ]
    },
    "tax_behavior": {
      "type": "string",
      "title": "Tax Behavior",
      "description": "Tax behavior for the price.",
      "examples": [
        "exclusive"
      ]
    }
  },
  "type": "object",
  "required": [
    "currency",
    "unit_amount",
    "tax_behavior"
  ],
  "title": "FormattedPrice"
}
```

### FormattedPriceListing

```json
{
  "properties": {
    "billing_cycle": {
      "type": "string",
      "enum": [
        "monthly",
        "yearly"
      ],
      "title": "Billing Cycle",
      "description": "Billing cycle for the listed prices.",
      "examples": [
        "monthly"
      ]
    },
    "prices": {
      "items": {
        "$ref": "#/components/schemas/FormattedPrice"
      },
      "type": "array",
      "title": "Prices",
      "description": "Prices available for the billing cycle.",
      "examples": [
        [
          {
            "currency": "USD",
            "tax_behavior": "exclusive",
            "unit_amount": 600
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "billing_cycle",
    "prices"
  ],
  "title": "FormattedPriceListing"
}
```

### Hdyhau

```json
{
  "type": "string",
  "enum": [
    "friend",
    "social_media",
    "ai_chatbot",
    "search_engine",
    "app_store",
    "other"
  ]
}
```

### Industry

```json
{
  "type": "string",
  "enum": [
    "agriculture",
    "arts_entertainment",
    "automotive",
    "banking_financial_services",
    "construction",
    "consulting",
    "consumer_goods",
    "education",
    "energy_utilities",
    "food_beverages",
    "government_public_sector",
    "healthcare_life_sciences",
    "information_technology",
    "insurance",
    "legal_services",
    "manufacturing",
    "media_communications",
    "non_profit",
    "pharmaceuticals",
    "real_estate",
    "retail_wholesale",
    "telecommunications",
    "transportation_logistics",
    "travel_hospitality",
    "other"
  ]
}
```

### IntendedTeamSize

```json
{
  "type": "string",
  "enum": [
    "size_1",
    "size_2_to_10",
    "size_11_to_50",
    "more_than_50",
    "size_2_to_7",
    "size_8_to_15",
    "size_16_to_24",
    "more_than_25"
  ]
}
```

### InviteWorkspaceUsersResponse

```json
{
  "properties": {
    "invited_emails": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Invited Emails",
      "description": "Emails successfully invited to the workspace.",
      "examples": [
        [
          "teammate@example.com"
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "invited_emails"
  ],
  "title": "InviteWorkspaceUsersResponse"
}
```

### MemberView

```json
{
  "properties": {
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "String ID of the workspace member.",
      "examples": [
        "1234567"
      ]
    },
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "String ID of the workspace.",
      "examples": [
        "123456"
      ]
    },
    "user_email": {
      "type": "string",
      "title": "User Email",
      "description": "Email address of the workspace member.",
      "examples": [
        "example@email.org"
      ]
    },
    "full_name": {
      "type": "string",
      "title": "Full Name",
      "description": "Full name of the workspace member.",
      "examples": [
        "Dain Ironfoot"
      ]
    },
    "timezone": {
      "type": "string",
      "title": "Timezone",
      "description": "Timezone of the workspace member.",
      "examples": [
        "Europe/Madrid"
      ]
    },
    "role": {
      "$ref": "#/components/schemas/WorkspaceRole",
      "description": "Workspace role assigned to the member.",
      "examples": [
        "MEMBER"
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
      "description": "Avatar image ID of the workspace member, or `null`.",
      "examples": [
        "abc123"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the workspace member is deleted.",
      "default": false,
      "examples": [
        false
      ]
    },
    "is_agent": {
      "type": "boolean",
      "title": "Is Agent",
      "description": "Whether the workspace member is an agent, such as `@todoist`, rather than a person. Only present when `true`.",
      "default": false,
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "required": [
    "user_id",
    "workspace_id",
    "user_email",
    "full_name",
    "timezone",
    "role"
  ],
  "title": "MemberView"
}
```

### OrganizationSize

```json
{
  "type": "string",
  "enum": [
    "size_1",
    "size_2_to_10",
    "size_11_to_50",
    "size_51_to_100",
    "size_101_to_250",
    "size_51_to_250",
    "more_than_250"
  ]
}
```

### PlanPrice

```json
{
  "properties": {
    "amount": {
      "type": "string",
      "title": "Amount",
      "description": "Formatted plan price amount.",
      "examples": [
        "600"
      ]
    },
    "raw_amount": {
      "type": "number",
      "title": "Raw Amount",
      "description": "Raw plan price amount in the currency's smallest unit.",
      "examples": [
        600.0
      ]
    },
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO 4217 currency code.",
      "examples": [
        "USD"
      ]
    },
    "billing_cycle": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "monthly",
            "yearly"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Billing Cycle",
      "description": "Billing cycle for the plan price, or `null` if unknown.",
      "examples": [
        "monthly"
      ]
    },
    "tax_behavior": {
      "type": "string",
      "enum": [
        "exclusive",
        "inclusive",
        "unspecified"
      ],
      "title": "Tax Behavior",
      "description": "Tax behavior for the plan price.",
      "examples": [
        "exclusive"
      ]
    }
  },
  "type": "object",
  "required": [
    "amount",
    "raw_amount",
    "currency",
    "billing_cycle",
    "tax_behavior"
  ],
  "title": "PlanPrice"
}
```

### PlanType

```json
{
  "type": "string",
  "enum": [
    "STARTER",
    "BUSINESS"
  ],
  "title": "PlanType",
  "description": "The plan the workspace is currently on",
  "x-enumDescriptions": {
    "BUSINESS": "Business",
    "STARTER": "Starter"
  }
}
```

### ProductApp

```json
{
  "type": "string",
  "enum": [
    "todoist",
    "comms",
    "automations"
  ],
  "title": "ProductApp",
  "description": "Product name as first-party clients spell it on the wire.",
  "x-enumDescriptions": {
    "automations": "Automations",
    "comms": "Comms",
    "todoist": "Todoist"
  }
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

### ProjectSortPreference

```json
{
  "type": "string",
  "enum": [
    "MANUAL",
    "A_TO_Z",
    "Z_TO_A"
  ],
  "title": "ProjectSortPreference",
  "description": "User preference for workspace project sorting",
  "x-enumDescriptions": {
    "A_TO_Z": "A To Z",
    "MANUAL": "Manual",
    "Z_TO_A": "Z To A"
  }
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

### ProjectViewStyle

```json
{
  "type": "string",
  "enum": [
    "LIST",
    "BOARD",
    "CALENDAR",
    "TABLE"
  ],
  "title": "ProjectViewStyle",
  "x-enumDescriptions": {
    "BOARD": "Board",
    "CALENDAR": "Calendar",
    "LIST": "List",
    "TABLE": "Table"
  }
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

### PublicWorkspaceProperties

```json
{
  "properties": {
    "industry": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Industry"
        },
        {
          "type": "null"
        }
      ],
      "description": "Industry selected for the workspace.",
      "examples": [
        "information_technology"
      ]
    },
    "department": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Department"
        },
        {
          "type": "null"
        }
      ],
      "description": "Department selected for the workspace.",
      "examples": [
        "engineering"
      ]
    },
    "organization_size": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/OrganizationSize"
        },
        {
          "type": "null"
        }
      ],
      "description": "Organization size selected for the workspace.",
      "examples": [
        "size_11_to_50"
      ]
    },
    "intended_team_size": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/IntendedTeamSize"
        },
        {
          "type": "null"
        }
      ],
      "description": "Number of collaborators the creator intends to work with, selected during onboarding.",
      "examples": [
        "size_2_to_10"
      ]
    },
    "type_of_work": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/TypeOfWork"
        },
        {
          "type": "null"
        }
      ],
      "description": "Type of work the team primarily does, selected during onboarding.",
      "examples": [
        "projects"
      ]
    },
    "creator_role": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CreatorRole"
        },
        {
          "type": "null"
        }
      ],
      "description": "Role selected by the workspace creator.",
      "examples": [
        "owner_founder"
      ]
    },
    "region": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Region",
      "description": "2-letter continent code, possible values are `AF`, `AS`, `EU`, `NA`, `SA`, `OC`, and `AN`.",
      "examples": [
        "EU"
      ]
    },
    "country": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Country",
      "description": "2-letter ISO 3166-1 alpha-2 country code.",
      "examples": [
        "ES"
      ]
    },
    "default_access_level": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/DefaultAccessLevel"
        },
        {
          "type": "null"
        }
      ],
      "description": "Default access level for new workspace projects.",
      "default": "team",
      "examples": [
        "team"
      ]
    },
    "desktop_workspace_modal": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/DesktopWorkspaceModal"
        },
        {
          "type": "null"
        }
      ],
      "description": "Desktop workspace modal variant shown to the user.",
      "examples": [
        "TRIAL_OFFER"
      ]
    },
    "beta_enabled": {
      "type": "boolean",
      "title": "Beta Enabled",
      "description": "Whether beta features are enabled for the workspace.",
      "default": false,
      "examples": [
        false
      ]
    },
    "acquisition_source": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/AcquisitionSource"
        },
        {
          "type": "null"
        }
      ],
      "description": "Workspace acquisition source.",
      "examples": [
        "high_paid_channel"
      ]
    },
    "team_acquisition_cohort": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/TeamAcquisitionCohort"
        },
        {
          "type": "null"
        }
      ],
      "description": "Team acquisition cohort assigned to the workspace.",
      "examples": [
        "type_1_unaware"
      ]
    },
    "acquisition_loop_last_show": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Acquisition Loop Last Show",
      "description": "Date when the acquisition loop was last shown in YYYY-MM-DD format.",
      "examples": [
        "2025-02-12"
      ]
    },
    "hdyhau": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/Hdyhau"
        },
        {
          "type": "null"
        }
      ],
      "description": "How did you hear about us marketing attribution field.",
      "examples": [
        "search_engine"
      ]
    },
    "onboarding_filter_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Onboarding Filter Id",
      "description": "ID of the workspace assigned filter created during onboarding.",
      "examples": [
        1234
      ]
    },
    "created_platform": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CreatedPlatform"
        },
        {
          "type": "null"
        }
      ],
      "description": "Platform used to create this workspace.",
      "examples": [
        "web"
      ]
    }
  },
  "type": "object",
  "title": "PublicWorkspaceProperties",
  "description": "Client-settable workspace properties.\n\nTrusted-only provenance fields are rejected outright (including\nexplicit nulls) so the public create/update surfaces can neither opt\na workspace into import behavior nor clear its provenance."
}
```

### RemoveWorkspaceUserResponse

```json
{
  "properties": {
    "status": {
      "type": "string",
      "const": "ok",
      "title": "Status",
      "description": "Always `ok` on success.",
      "default": "ok",
      "examples": [
        "ok"
      ]
    }
  },
  "type": "object",
  "title": "RemoveWorkspaceUserResponse"
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

### TeamAcquisitionCohort

```json
{
  "type": "string",
  "enum": [
    "type_1_unaware",
    "type_2_unconvinced",
    "type_3_reverted"
  ],
  "title": "TeamAcquisitionCohort",
  "x-enumDescriptions": {
    "type_1_unaware": "Type 1 Unaware",
    "type_2_unconvinced": "Type 2 Unconvinced",
    "type_3_reverted": "Type 3 Reverted"
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

### TrialGrantSource

```json
{
  "type": "string",
  "enum": [
    "twist_migration"
  ],
  "title": "TrialGrantSource",
  "x-enumDescriptions": {
    "twist_migration": "Twist Migration"
  }
}
```

### TypeOfWork

```json
{
  "type": "string",
  "enum": [
    "projects",
    "processes",
    "both"
  ]
}
```

### WorkspaceInvitationView

```json
{
  "properties": {
    "inviter_id": {
      "type": "string",
      "title": "Inviter Id",
      "description": "ID of the user user who sent the invitation",
      "examples": [
        "1029384756"
      ]
    },
    "user_email": {
      "type": "string",
      "title": "User Email",
      "description": "The invited person's email.",
      "examples": [
        "foo@example.com"
      ]
    },
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "ID of the workspace",
      "examples": [
        "12345"
      ]
    },
    "role": {
      "$ref": "#/components/schemas/WorkspaceRole",
      "description": "Workspace role granted by the invitation.",
      "examples": [
        "MEMBER"
      ]
    },
    "id": {
      "type": "string",
      "title": "Id",
      "description": "The ID of the invitation",
      "default": "0",
      "examples": [
        "234"
      ]
    },
    "app": {
      "$ref": "#/components/schemas/ProductApp",
      "description": "Product the invitation was sent from.",
      "default": "todoist",
      "examples": [
        "comms"
      ]
    }
  },
  "type": "object",
  "required": [
    "inviter_id",
    "user_email",
    "workspace_id",
    "role"
  ],
  "title": "WorkspaceInvitationView"
}
```

### WorkspaceLimitsSyncView

```json
{
  "properties": {
    "current": {
      "$ref": "#/components/schemas/WorkspacePlanLimitSettings",
      "description": "Limits of the workspace's current plan."
    },
    "next": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/WorkspacePlanLimitSettings"
        },
        {
          "type": "null"
        }
      ],
      "description": "Limits of the next upgrade plan, or `null` if the workspace is already on the highest plan."
    }
  },
  "type": "object",
  "required": [
    "current",
    "next"
  ],
  "title": "WorkspaceLimitsSyncView",
  "description": "Current and next plan limits returned for a workspace in sync responses."
}
```

### WorkspacePlanLimitSettings

```json
{
  "properties": {
    "plan_name": {
      "type": "string",
      "title": "Plan Name",
      "description": "Name of the plan these limits belong to.",
      "examples": [
        "pro"
      ]
    },
    "max_projects": {
      "type": "integer",
      "title": "Max Projects",
      "description": "Maximum number of active projects.",
      "examples": [
        300
      ]
    },
    "max_collaborators": {
      "type": "integer",
      "title": "Max Collaborators",
      "description": "Maximum number of collaborators in one project.",
      "examples": [
        25
      ]
    },
    "upload_limit_mb": {
      "type": "integer",
      "title": "Upload Limit Mb",
      "description": "Maximum size of a file upload, in MB.",
      "examples": [
        100
      ]
    },
    "automatic_backups": {
      "type": "boolean",
      "title": "Automatic Backups",
      "description": "Whether automatic backups are made.",
      "examples": [
        true
      ]
    },
    "reminders": {
      "type": "boolean",
      "title": "Reminders",
      "description": "Whether reminders can be added.",
      "examples": [
        true
      ]
    },
    "reminders_at_due": {
      "type": "boolean",
      "title": "Reminders At Due",
      "description": "Whether reminders at the due time, the limited form, can be added.",
      "examples": [
        true
      ]
    },
    "max_guests_per_workspace": {
      "type": "integer",
      "title": "Max Guests Per Workspace",
      "description": "Maximum number of guests in a workspace, personal or team.",
      "examples": [
        100
      ]
    },
    "max_folders_per_workspace": {
      "type": "integer",
      "title": "Max Folders Per Workspace",
      "description": "Maximum number of folders in a workspace.",
      "examples": [
        100
      ]
    },
    "advanced_permissions": {
      "type": "boolean",
      "title": "Advanced Permissions",
      "description": "Whether project members can be given granular permissions such as comment-only or view-only.",
      "examples": [
        true
      ]
    },
    "durations": {
      "type": "boolean",
      "title": "Durations",
      "description": "Whether task durations can be set.",
      "examples": [
        true
      ]
    },
    "calendar_layout": {
      "type": "boolean",
      "title": "Calendar Layout",
      "description": "Whether the calendar layout is available for projects.",
      "default": false,
      "examples": [
        true
      ]
    },
    "table_layout": {
      "type": "boolean",
      "title": "Table Layout",
      "description": "Whether the table layout can be picked as a view option.",
      "default": false,
      "examples": [
        true
      ]
    },
    "email_forwarding": {
      "type": "boolean",
      "title": "Email Forwarding",
      "description": "Whether tasks and comments can be added by email.",
      "default": true,
      "examples": [
        true
      ]
    },
    "email_ai_forwarding": {
      "type": "boolean",
      "title": "Email Ai Forwarding",
      "description": "Whether forwarded emails are parsed with AI.",
      "default": false,
      "examples": [
        true
      ]
    },
    "project_icons": {
      "type": "boolean",
      "title": "Project Icons",
      "description": "Whether projects can have a custom icon.",
      "default": false,
      "examples": [
        true
      ]
    },
    "max_live_notifications": {
      "type": "integer",
      "title": "Max Live Notifications",
      "description": "Maximum number of live notifications kept.",
      "default": 500,
      "examples": [
        500
      ]
    },
    "max_monthly_capture": {
      "type": "integer",
      "title": "Max Monthly Capture",
      "description": "Maximum number of task capture sessions per calendar month.",
      "default": 10,
      "examples": [
        10
      ]
    },
    "max_monthly_ramble": {
      "type": "integer",
      "title": "Max Monthly Ramble",
      "description": "Deprecated mirror of `max_monthly_capture`.",
      "default": 10,
      "examples": [
        10
      ]
    },
    "admin_tools": {
      "type": "boolean",
      "title": "Admin Tools",
      "description": "Whether the workspace has the admin and member roles.",
      "default": false,
      "examples": [
        true
      ]
    },
    "security_controls": {
      "type": "boolean",
      "title": "Security Controls",
      "description": "Whether the workspace has security settings.",
      "default": false,
      "examples": [
        true
      ]
    },
    "max_filters": {
      "type": "integer",
      "title": "Max Filters",
      "description": "Maximum number of shared team filters.",
      "default": 3,
      "examples": [
        3
      ]
    },
    "max_workspace_users": {
      "type": "integer",
      "title": "Max Workspace Users",
      "description": "Maximum number of members in the workspace.",
      "default": 0,
      "examples": [
        500
      ]
    },
    "max_workspaces": {
      "type": "integer",
      "title": "Max Workspaces",
      "description": "Maximum number of workspaces a user can be a member of.",
      "default": 50,
      "examples": [
        50
      ]
    },
    "max_workspace_templates": {
      "type": "integer",
      "title": "Max Workspace Templates",
      "description": "Maximum number of templates in the workspace.",
      "default": 100,
      "examples": [
        100
      ]
    },
    "project_insights": {
      "type": "boolean",
      "title": "Project Insights",
      "description": "Whether the full Project Insights (sidebar, progress, completed counts and Project Health) are available.",
      "default": false,
      "examples": [
        true
      ]
    }
  },
  "type": "object",
  "required": [
    "plan_name",
    "max_projects",
    "max_collaborators",
    "upload_limit_mb",
    "automatic_backups",
    "reminders",
    "reminders_at_due",
    "max_guests_per_workspace",
    "max_folders_per_workspace",
    "advanced_permissions",
    "durations"
  ],
  "title": "WorkspacePlanLimitSettings"
}
```

### WorkspaceProjectView

```json
{
  "properties": {
    "initiated_by_uid": {
      "type": "integer",
      "title": "Initiated By Uid",
      "description": "Numeric ID of the user requesting the workspace project.",
      "examples": [
        1234567
      ]
    },
    "project_id": {
      "type": "string",
      "title": "Project Id",
      "description": "String ID of the project.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "workspace_id": {
      "type": "integer",
      "title": "Workspace Id",
      "description": "Numeric ID of the workspace that contains the project.",
      "examples": [
        123456
      ]
    },
    "public_access": {
      "type": "boolean",
      "title": "Public Access",
      "description": "Whether the project is publicly accessible.",
      "examples": [
        false
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
    "folder_id": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Folder Id",
      "description": "Numeric ID of the folder that contains the project, or `null`.",
      "examples": [
        1234
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
      "default": false,
      "examples": [
        false
      ]
    },
    "is_archived": {
      "type": "boolean",
      "title": "Is Archived",
      "description": "Whether the workspace project is archived.",
      "default": false,
      "examples": [
        false
      ]
    },
    "archived_timestamp": {
      "type": "integer",
      "title": "Archived Timestamp",
      "description": "Unix timestamp for when the project was archived, or 0 if active.",
      "default": 0,
      "examples": [
        0
      ]
    },
    "archived_date": {
      "anyOf": [
        {
          "type": "string",
          "format": "date-time"
        },
        {
          "type": "null"
        }
      ],
      "title": "Archived Date",
      "description": "Date and time when the project was archived, or `null` if active.",
      "examples": [
        null
      ]
    },
    "is_frozen": {
      "type": "boolean",
      "title": "Is Frozen",
      "description": "Whether the project is frozen and cannot be modified.",
      "default": false,
      "examples": [
        false
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Project name.",
      "default": "",
      "examples": [
        "Marketing"
      ]
    },
    "color": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Color",
      "description": "Numeric project color code.",
      "default": 47,
      "examples": [
        47
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
      "description": "Project icon name, or `null` for the default icon.",
      "examples": [
        "lightbulb"
      ]
    },
    "view_style": {
      "$ref": "#/components/schemas/ProjectViewStyle",
      "description": "Project view style.",
      "default": "list",
      "examples": [
        "list"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Project description.",
      "default": "",
      "examples": [
        "Marketing launch tasks"
      ]
    },
    "status": {
      "$ref": "#/components/schemas/ProjectStatus",
      "description": "Project status.",
      "default": "IN_PROGRESS",
      "examples": [
        "in_progress"
      ]
    },
    "default_order": {
      "type": "integer",
      "title": "Default Order",
      "description": "Default order of the project in workspace views.",
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
    "is_project_insights_enabled": {
      "type": "boolean",
      "title": "Is Project Insights Enabled",
      "description": "Whether project insights are enabled for the project.",
      "default": true,
      "examples": [
        true
      ]
    },
    "_role": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/CollaboratorRole"
        },
        {
          "type": "null"
        }
      ],
      "title": "Role"
    }
  },
  "type": "object",
  "required": [
    "initiated_by_uid",
    "project_id",
    "workspace_id",
    "public_access",
    "access"
  ],
  "title": "WorkspaceProjectView"
}
```

### WorkspaceRole

```json
{
  "type": "string",
  "enum": [
    "ADMIN",
    "MEMBER",
    "GUEST"
  ],
  "title": "WorkspaceRole",
  "description": "Role of the user inside the workspace",
  "x-enumDescriptions": {
    "ADMIN": "Admin",
    "GUEST": "Guest",
    "MEMBER": "Member"
  }
}
```

### WorkspaceSyncView

```json
{
  "properties": {
    "id": {
      "type": "string",
      "title": "Id",
      "description": "String ID of the workspace.",
      "examples": [
        "123456"
      ]
    },
    "name": {
      "type": "string",
      "title": "Name",
      "description": "Workspace name.",
      "examples": [
        "Todoist"
      ]
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Workspace description.",
      "examples": [
        "Team workspace"
      ]
    },
    "creator_id": {
      "type": "string",
      "title": "Creator Id",
      "description": "String ID of the user who created the workspace.",
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
      "description": "Date and time when the workspace was created.",
      "examples": [
        "2025-01-15T10:30:00Z"
      ]
    },
    "is_deleted": {
      "type": "boolean",
      "title": "Is Deleted",
      "description": "Whether the workspace is deleted.",
      "examples": [
        false
      ]
    },
    "is_collapsed": {
      "type": "boolean",
      "title": "Is Collapsed",
      "description": "Whether the workspace is collapsed in the user's sidebar.",
      "examples": [
        false
      ]
    },
    "role": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/WorkspaceRole"
        },
        {
          "type": "null"
        }
      ],
      "description": "Role of the current user in the workspace, or `null` if they are not part of it.",
      "examples": [
        "ADMIN"
      ]
    },
    "plan": {
      "$ref": "#/components/schemas/PlanType",
      "description": "The plan the workspace is currently on.",
      "examples": [
        "STARTER"
      ]
    },
    "pending_invitations": {
      "items": {
        "type": "string"
      },
      "type": "array",
      "title": "Pending Invitations",
      "description": "Emails with a pending invitation to the workspace. Only populated for workspace members.",
      "examples": [
        [
          "foo@example.com"
        ]
      ]
    },
    "is_link_sharing_enabled": {
      "anyOf": [
        {
          "type": "boolean"
        },
        {
          "type": "null"
        }
      ],
      "title": "Is Link Sharing Enabled",
      "description": "Whether joining the workspace via a link is enabled. `null` for non-members.",
      "examples": [
        true
      ]
    },
    "is_guest_allowed": {
      "type": "boolean",
      "title": "Is Guest Allowed",
      "description": "Whether guests are allowed in the workspace.",
      "examples": [
        true
      ]
    },
    "is_trial_pending": {
      "type": "boolean",
      "title": "Is Trial Pending",
      "description": "Whether the workspace is waiting for trial confirmation.",
      "examples": [
        false
      ]
    },
    "invite_code": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Invite Code",
      "description": "Opaque code to invite users to the workspace, or `null`.",
      "examples": [
        "abc123"
      ]
    },
    "limits": {
      "$ref": "#/components/schemas/WorkspaceLimitsSyncView",
      "description": "Current and next plan limits for the workspace."
    },
    "current_member_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Current Member Count",
      "description": "Number of members in the workspace. `null` for non-members.",
      "examples": [
        5
      ]
    },
    "current_active_projects": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Current Active Projects",
      "description": "Number of active projects in the workspace. `null` for non-members.",
      "examples": [
        10
      ]
    },
    "domain_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Domain Name",
      "description": "Email domain associated with the workspace, or `null`.",
      "examples": [
        "doist.com"
      ]
    },
    "domain_discovery": {
      "type": "boolean",
      "title": "Domain Discovery",
      "description": "Whether users with a matching email domain can discover the workspace.",
      "examples": [
        false
      ]
    },
    "restrict_email_domains": {
      "type": "boolean",
      "title": "Restrict Email Domains",
      "description": "Whether membership is restricted to allowed email domains.",
      "examples": [
        false
      ]
    },
    "admin_sorting_applied": {
      "type": "boolean",
      "title": "Admin Sorting Applied",
      "description": "Whether an admin has applied custom project sorting.",
      "examples": [
        false
      ]
    },
    "user_sorting_applied": {
      "type": "boolean",
      "title": "User Sorting Applied",
      "description": "Whether the current user has sorted their workspace sidebar before.",
      "examples": [
        false
      ]
    },
    "project_sort_preference": {
      "$ref": "#/components/schemas/ProjectSortPreference",
      "description": "The current user's project sort preference.",
      "examples": [
        "MANUAL"
      ]
    },
    "default_collaborators": {
      "$ref": "#/components/schemas/DefaultCollaboratorsSyncView",
      "description": "Default collaborators for new workspace projects."
    },
    "properties": {
      "additionalProperties": true,
      "type": "object",
      "description": "Additional workspace properties."
    },
    "logo_s640": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Logo S640",
      "description": "URL of the workspace logo (640px).",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/logo_s640.jpg"
      ]
    },
    "logo_big": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Logo Big",
      "description": "URL of the workspace logo (195px).",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/logo_big.jpg"
      ]
    },
    "logo_medium": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Logo Medium",
      "description": "URL of the workspace logo (60px).",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/logo_medium.jpg"
      ]
    },
    "logo_small": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Logo Small",
      "description": "URL of the workspace logo (35px).",
      "examples": [
        "https://dcff1xvirvpfp.cloudfront.net/logo_small.jpg"
      ]
    },
    "current_template_count": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Current Template Count",
      "description": "Number of active templates in the workspace. Only returned for workspace members.",
      "examples": [
        3
      ]
    },
    "pending_invites_by_type": {
      "anyOf": [
        {
          "additionalProperties": {
            "type": "integer"
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Pending Invites By Type",
      "description": "Number of pending invitations by workspace role. Only returned for workspace members.",
      "examples": [
        {
          "admin_count": 0,
          "guest_count": 0,
          "member_count": 2
        }
      ]
    },
    "member_count_by_type": {
      "anyOf": [
        {
          "additionalProperties": {
            "type": "integer"
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Member Count By Type",
      "description": "Number of members by workspace role. Only returned for workspace members.",
      "examples": [
        {
          "admin_count": 1,
          "guest_count": 0,
          "member_count": 4
        }
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "name",
    "description",
    "creator_id",
    "created_at",
    "is_deleted",
    "is_collapsed",
    "role",
    "plan",
    "pending_invitations",
    "is_link_sharing_enabled",
    "is_guest_allowed",
    "is_trial_pending",
    "invite_code",
    "limits",
    "current_member_count",
    "current_active_projects",
    "domain_name",
    "domain_discovery",
    "restrict_email_domains",
    "admin_sorting_applied",
    "user_sorting_applied",
    "project_sort_preference",
    "default_collaborators",
    "properties"
  ],
  "title": "WorkspaceSyncView",
  "description": "A class with fields representing a WorkspaceView which will be returned to\nclients in a sync (or sync-like) response.\n\nAlways construct instances through `from_view()`: `__json__` serializes\nwith `exclude_unset=True` so that fields `from_view()` deliberately leaves\nunset (member-only fields for non-members, `logo_*` fields for workspaces\nwithout a logo) are omitted from the response, exactly like the historical\n`WorkspaceView.__json__` output."
}
```

### WorkspaceUserView

```json
{
  "properties": {
    "user_id": {
      "type": "string",
      "title": "User Id",
      "description": "String ID of the workspace user.",
      "examples": [
        "1234567"
      ]
    },
    "workspace_id": {
      "type": "string",
      "title": "Workspace Id",
      "description": "String ID of the workspace.",
      "examples": [
        "123456"
      ]
    },
    "role": {
      "$ref": "#/components/schemas/WorkspaceRole",
      "description": "Workspace role assigned to the user.",
      "default": "MEMBER",
      "examples": [
        "MEMBER"
      ]
    },
    "custom_sorting_applied": {
      "type": "boolean",
      "title": "Custom Sorting Applied",
      "description": "Whether the user has custom project sorting enabled.",
      "default": false,
      "examples": [
        false
      ]
    },
    "project_sort_preference": {
      "$ref": "#/components/schemas/ProjectSortPreference",
      "description": "Project sorting preference for the user.",
      "default": "MANUAL",
      "examples": [
        "MANUAL"
      ]
    }
  },
  "type": "object",
  "required": [
    "user_id",
    "workspace_id"
  ],
  "title": "WorkspaceUserView"
}
```

### _PlanDetailsResponse

```json
{
  "properties": {
    "current_member_count": {
      "type": "integer",
      "title": "Current Member Count",
      "description": "Current number of active workspace members.",
      "examples": [
        10
      ]
    },
    "current_plan": {
      "type": "string",
      "enum": [
        "Business",
        "Starter"
      ],
      "title": "Current Plan",
      "description": "Current workspace plan.",
      "examples": [
        "Business"
      ]
    },
    "current_plan_status": {
      "type": "string",
      "enum": [
        "Active",
        "Downgraded",
        "Cancelled",
        "NeverSubscribed"
      ],
      "title": "Current Plan Status",
      "description": "Current workspace plan status.",
      "examples": [
        "Active"
      ]
    },
    "downgrade_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Downgrade At",
      "description": "Date when the workspace downgrades, or `null` if not scheduled.",
      "examples": [
        null
      ]
    },
    "current_active_projects": {
      "type": "integer",
      "title": "Current Active Projects",
      "description": "Current number of active workspace projects.",
      "examples": [
        12
      ]
    },
    "maximum_active_projects": {
      "type": "integer",
      "title": "Maximum Active Projects",
      "description": "Maximum number of active projects allowed by the plan.",
      "examples": [
        500
      ]
    },
    "price_list": {
      "items": {
        "$ref": "#/components/schemas/FormattedPriceListing"
      },
      "type": "array",
      "title": "Price List",
      "description": "Available plan prices for the workspace country.",
      "examples": [
        [
          {
            "billing_cycle": "monthly",
            "prices": [
              {
                "currency": "USD",
                "tax_behavior": "exclusive",
                "unit_amount": 600
              }
            ]
          }
        ]
      ]
    },
    "workspace_id": {
      "type": "integer",
      "title": "Workspace Id",
      "description": "ID of the workspace.",
      "examples": [
        123456
      ]
    },
    "is_trialing": {
      "type": "boolean",
      "title": "Is Trialing",
      "description": "Whether the workspace is currently trialing a paid plan.",
      "examples": [
        false
      ]
    },
    "trial_ends_at": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Trial Ends At",
      "description": "Date when the current trial ends, or `null` if not trialing.",
      "examples": [
        null
      ]
    },
    "cancel_at_period_end": {
      "type": "boolean",
      "title": "Cancel At Period End",
      "description": "Whether the plan cancels at the end of the billing period.",
      "examples": [
        false
      ]
    },
    "has_trialed": {
      "type": "boolean",
      "title": "Has Trialed",
      "description": "Whether the workspace has already used a trial.",
      "examples": [
        true
      ]
    },
    "plan_price": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PlanPrice"
        },
        {
          "type": "null"
        }
      ],
      "description": "Current subscription price, or `null` when there is no active subscription.",
      "examples": [
        null
      ]
    },
    "has_billing_portal": {
      "type": "boolean",
      "title": "Has Billing Portal",
      "description": "Whether a billing portal is available for the workspace.",
      "examples": [
        true
      ]
    },
    "has_billing_portal_switch_to_annual": {
      "type": "boolean",
      "title": "Has Billing Portal Switch To Annual",
      "description": "Whether the billing portal can switch the subscription to annual billing.",
      "examples": [
        false
      ]
    },
    "trial_grant": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/_TrialGrantView"
        },
        {
          "type": "null"
        }
      ],
      "description": "The trial the workspace's next Business checkout starts instead of any other, even after an earlier trial, e.g. a migrated team's prepaid time. `null` without one.",
      "examples": [
        null
      ]
    }
  },
  "type": "object",
  "required": [
    "current_member_count",
    "current_plan",
    "current_plan_status",
    "downgrade_at",
    "current_active_projects",
    "maximum_active_projects",
    "price_list",
    "workspace_id",
    "is_trialing",
    "trial_ends_at",
    "cancel_at_period_end",
    "has_trialed",
    "plan_price",
    "has_billing_portal",
    "has_billing_portal_switch_to_annual",
    "trial_grant"
  ],
  "title": "_PlanDetailsResponse"
}
```

### _ProjectsResponse

```json
{
  "properties": {
    "has_more": {
      "type": "boolean",
      "title": "Has More",
      "description": "Whether another page of results is available.",
      "examples": [
        false
      ]
    },
    "next_cursor": {
      "type": "string",
      "title": "Next Cursor",
      "description": "Cursor for the next page of results.",
      "examples": [
        "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
      ]
    },
    "workspace_projects": {
      "items": {
        "$ref": "#/components/schemas/WorkspaceProjectView"
      },
      "type": "array",
      "title": "Workspace Projects",
      "description": "Workspace projects in the current page.",
      "examples": [
        [
          {
            "access": {
              "configuration": {},
              "visibility": "team"
            },
            "can_assign_tasks": true,
            "color": "charcoal",
            "default_order": 1,
            "description": "Marketing launch tasks",
            "folder_id": "1234",
            "icon": "lightbulb",
            "id": "6XGgm6PHrGgMpCFX",
            "is_frozen": false,
            "is_invite_only": false,
            "is_pending_default_collaborator_invites": false,
            "name": "Marketing",
            "public_access": false,
            "public_key": "ab12cd34ef56",
            "role": "ADMIN",
            "status": "in_progress",
            "view_style": "list"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "has_more",
    "workspace_projects"
  ],
  "title": "_ProjectsResponse"
}
```

### _TrialGrantView

```json
{
  "properties": {
    "source": {
      "$ref": "#/components/schemas/TrialGrantSource",
      "description": "Where the grant comes from.",
      "examples": [
        "twist_migration"
      ]
    },
    "trial_days": {
      "type": "integer",
      "title": "Trial Days",
      "description": "Free days the next Business checkout starts, counted from activation.",
      "examples": [
        135
      ]
    },
    "bonus_days": {
      "type": "integer",
      "title": "Bonus Days",
      "description": "Of `trial_days`, the days given on top of the time the team carried over, e.g. 90 for a migrated Twist team.",
      "examples": [
        90
      ]
    }
  },
  "type": "object",
  "required": [
    "source",
    "trial_days",
    "bonus_days"
  ],
  "title": "_TrialGrantView"
}
```

### _UsersResponse

```json
{
  "properties": {
    "has_more": {
      "type": "boolean",
      "title": "Has More",
      "description": "Whether more results are available.",
      "examples": [
        false
      ]
    },
    "next_cursor": {
      "type": "string",
      "title": "Next Cursor",
      "description": "Cursor for the next page of results.",
      "examples": [
        "eyJpZCI6MTIzfQ"
      ]
    },
    "workspace_users": {
      "items": {
        "$ref": "#/components/schemas/MemberView"
      },
      "type": "array",
      "title": "Workspace Users",
      "description": "Workspace users in the current page.",
      "examples": [
        [
          {
            "full_name": "Dain Ironfoot",
            "image_id": "abc123",
            "is_agent": true,
            "is_deleted": false,
            "role": "MEMBER",
            "timezone": "Europe/Madrid",
            "user_email": "example@email.org",
            "user_id": "1234567",
            "workspace_id": "123456"
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "has_more",
    "workspace_users"
  ],
  "title": "_UsersResponse"
}
```
