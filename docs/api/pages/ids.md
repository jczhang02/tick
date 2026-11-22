# Ids

Endpoints related to ID mappings between v1 and v2

## Endpoints

### Id Mappings

`GET /api/v1/id_mappings/{obj_name}/{obj_ids}`

Translates IDs from v1 to v2 or vice versa.

IDs are not unique across object types, hence the need to specify the object type.

When V1 ids are provided, the function will return the corresponding V2
ids, if they exist, and vice versa.

When no objects are found, an empty list is returned.

```json
{
  "parameters": [
    {
      "name": "obj_name",
      "in": "path",
      "required": true,
      "schema": {
        "enum": [
          "sections",
          "tasks",
          "comments",
          "reminders",
          "location_reminders",
          "projects"
        ],
        "type": "string",
        "description": "Object type for the IDs to translate.",
        "examples": [
          "tasks"
        ],
        "title": "Obj Name"
      }
    },
    {
      "name": "obj_ids",
      "in": "path",
      "required": true,
      "schema": {
        "type": "string",
        "description": "A comma-separated list of IDs",
        "examples": [
          "6VfWjjjFg2xqX6Pa",
          "918273645",
          "6VfWjjjFg2xqX6Pa,6WMVPf8Hm8JP6mC8"
        ],
        "title": "Obj Ids"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/IDMappingsMultipleResponse"
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

### IDMapping

```json
{
  "properties": {
    "old_id": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/V1orV2IDStr"
        },
        {
          "type": "null"
        }
      ],
      "description": "Legacy ID for the object.",
      "examples": [
        "918273645"
      ]
    },
    "new_id": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/V1orV2IDStr"
        },
        {
          "type": "null"
        }
      ],
      "description": "Curent ID for the object.",
      "examples": [
        "6VfWjjjFg2xqX6Pa"
      ]
    }
  },
  "type": "object",
  "required": [
    "old_id",
    "new_id"
  ],
  "title": "IDMapping"
}
```

### IDMappingsMultipleResponse

```json
{
  "items": {
    "$ref": "#/components/schemas/IDMapping"
  },
  "type": "array"
}
```

### V1orV2IDStr

```json
{
  "type": "string"
}
```
