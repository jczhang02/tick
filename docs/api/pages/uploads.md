# Uploads

Availability of uploads functionality and the maximum size for a file
attachment are dependent on the current user plan. These values are indicated
by the `uploads` and `upload_limit_mb` properties of the user plan limits object.

Files can be uploaded to our servers and used as [File
Attachments](https://developer.todoist.com/api/v1/sync/comments.md) in [comments](https://developer.todoist.com/api/v1/comments.md).

## Endpoints

### Delete Upload

`DELETE /api/v1/uploads`

Delete an uploaded file.

The file must belong to the authenticated user. Returns `FILE_NOT_FOUND`
when the upload cannot be found.

```json
{
  "parameters": [
    {
      "name": "file_url",
      "in": "query",
      "required": true,
      "schema": {
        "type": "string",
        "minLength": 1,
        "description": "URL of the uploaded file to delete.",
        "examples": [
          "https://files.todoist.com/abc123/by/1234567/as/file.pdf"
        ],
        "title": "File Url"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "const": "ok",
            "type": "string",
            "title": "Response Delete Upload Api V1 Uploads Delete"
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

### Upload File

`POST /api/v1/uploads`

Upload a file to Todoist.

This endpoint accepts file uploads via two methods:

1. **Multipart form-data** (recommended):
   - Send the file as a form field with the actual file content
   - Optionally include `project_id` as another form field
   - The filename will be extracted from the Content-Disposition header

2. **Raw binary stream**:
   - Send the file content directly in the request body
   - Set `Content-Type` header to the file's MIME type
   - Set `X-File-Name` header with the desired filename
   - Optionally include `project_id` as a query parameter

The optional `project_id` parameter can be used to apply workspace-specific
upload limits when uploading to a workspace project.

```json
{
  "requestBody": {
    "required": true,
    "content": {
      "multipart/form-data": {
        "schema": {
          "$ref": "#/components/schemas/Body_upload_file_api_v1_uploads_post"
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
            "$ref": "#/components/schemas/UploadResult"
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

$ curl https://api.todoist.com/api/v1/uploads \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -F file=@/path/to/file.pdf
```

```curl

$ curl https://api.todoist.com/api/v1/uploads \
       -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
       -H "Content-Type: application/pdf" \
       -H "X-File-Name: document.pdf" \
       --data-binary @/path/to/file.pdf
```

## Schemas

The endpoints above refer to these schemas with `$ref`.

### Body_upload_file_api_v1_uploads_post

```json
{
  "properties": {
    "file_name": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "File Name",
      "description": "Name for the uploaded file. If not provided, will be extracted from Content-Disposition header (multipart) or X-File-Name header (raw binary).",
      "examples": [
        "document.pdf"
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
      "description": "Project ID to apply workspace-specific upload limits when uploading to a workspace project.",
      "examples": [
        "6XGgm6PHrGgMpCFX"
      ]
    },
    "file": {
      "type": "string",
      "contentMediaType": "application/octet-stream",
      "title": "File",
      "description": "The file to upload. Its size is limited by the user's plan, or by the workspace's plan when `project_id` is a workspace project."
    }
  },
  "type": "object",
  "required": [
    "file"
  ],
  "title": "Body_upload_file_api_v1_uploads_post"
}
```

### UploadResult

```json
{
  "properties": {
    "file_url": {
      "type": "string",
      "title": "File Url",
      "description": "URL of the uploaded file.",
      "examples": [
        "https://files.todoist.com/abc123/by/1234567/as/file.pdf"
      ]
    },
    "file_name": {
      "type": "string",
      "title": "File Name",
      "description": "Name of the uploaded file.",
      "examples": [
        "document.pdf"
      ]
    },
    "file_size": {
      "type": "integer",
      "title": "File Size",
      "description": "Size of the uploaded file in bytes.",
      "examples": [
        12345
      ]
    },
    "file_type": {
      "type": "string",
      "title": "File Type",
      "description": "MIME type of the uploaded file.",
      "examples": [
        "application/pdf"
      ]
    },
    "resource_type": {
      "type": "string",
      "title": "Resource Type",
      "description": "Detected resource type of the upload.",
      "examples": [
        "file"
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
      "description": "Image URL for image uploads, or `null` for non-image files.",
      "examples": [
        null
      ]
    },
    "image_width": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Image Width",
      "description": "Image width in pixels, or `null` for non-image files.",
      "examples": [
        1200
      ]
    },
    "image_height": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Image Height",
      "description": "Image height in pixels, or `null` for non-image files.",
      "examples": [
        800
      ]
    },
    "upload_state": {
      "type": "string",
      "enum": [
        "pending",
        "completed"
      ],
      "title": "Upload State",
      "description": "Upload processing state.",
      "default": "pending",
      "examples": [
        "completed"
      ]
    }
  },
  "type": "object",
  "required": [
    "file_url",
    "file_name",
    "file_size",
    "file_type",
    "resource_type"
  ],
  "title": "UploadResult"
}
```
