# Backups

_Availability of backups functionality is dependent on the current user plan. This value is indicated by the automatic_backups property of the user plan limits object._

## Endpoints

### Download Backup

`GET /api/v1/backups/download`

Download a backup archive.

This endpoint requires the `data:read_write` scope. The `backups:read`
scope only grants access to `GET /api/v1/backups`.

Validates that the requested backup belongs to the authenticated user and
redirects to a signed CloudFront URL that expires after one minute.

```json
{
  "parameters": [
    {
      "name": "file",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "description": "Backup file name returned in a backup download URL.",
        "examples": [
          "12345678901234567890123456789012.zip"
        ],
        "title": "File"
      }
    }
  ],
  "responses": {
    "302": {
      "description": "Redirect"
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

### Get Backups

`GET /api/v1/backups`

Todoist creates a backup archive of users' data on a daily basis. Backup
archives can also be accessed from the web app (Todoist Settings ->
Backups).

This endpoint accepts the `backups:read` scope, the `data:read_write`
scope, or a personal API token. The `backups:read` scope bypasses MFA and
only grants access to `GET /api/v1/backups`. When using `data:read_write`
or a personal API token for an account with MFA enabled, the MFA token is
required.

```json
{
  "parameters": [
    {
      "name": "mfa_token",
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
        "description": "MFA token required when using a `data:read_write` token for an account with MFA enabled.",
        "examples": [
          "123456"
        ],
        "title": "Mfa Token"
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
              "$ref": "#/components/schemas/_BackupResponse"
            },
            "title": "Response Get Backups Api V1 Backups Get"
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

### _BackupResponse

```json
{
  "properties": {
    "version": {
      "type": "string",
      "title": "Version",
      "description": "Date and time of the backup version",
      "examples": [
        "2025-02-13 02:03"
      ]
    },
    "url": {
      "type": "string",
      "title": "Url",
      "description": "Backup URL",
      "examples": [
        "https://api.todoist.com/api/v1/backups/download?file=12345678901234567890123456789012.zip"
      ]
    }
  },
  "type": "object",
  "required": [
    "version",
    "url"
  ],
  "title": "_BackupResponse"
}
```
