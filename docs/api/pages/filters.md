# Filters

Filters are created and updated via the `/sync` endpoint. They can also be deleted with the endpoint below.

## Endpoints

### Delete Filter

`DELETE /api/v1/filters/{filter_id}`

Delete a personal filter.

Only available on plans that include filters. Returns a not found error if
the filter doesn't exist, was already deleted, or belongs to another user.
Workspace filters can't be deleted with this endpoint.

```json
{
  "parameters": [
    {
      "name": "filter_id",
      "in": "path",
      "required": true,
      "schema": {
        "type": "integer",
        "description": "String ID of the filter",
        "examples": [
          "2370326476"
        ],
        "title": "Filter Id"
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
