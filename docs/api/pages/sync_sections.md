# Sync: Sections

> An example section object

```
{
    "id": "6Jf8VQXxpwv56VQ7",
    "name": "Groceries",
    "description": "Tasks for the launch plan",
    "project_id": "9Bw8VQXxpwv56ZY2",
    "section_order": 1,
    "order_key": "a1V",
    "is_collapsed": false,
    "user_id": "2671355",
    "is_deleted": false,
    "is_archived": false,
    "archived_at": null,
    "added_at": "2019-10-07T07:09:27.000000Z",
    "updated_at": "2019-10-07T07:09:27.000000Z"
}
```

#### Properties

| Property                | Description                                                                                           |
|-------------------------|-------------------------------------------------------------------------------------------------------|
| id _String_             | The ID of the section.                                                                                |
| name _String_           | The name of the section.                                                                              |
| description _String_    | The description of the section (or `null` if it has none).                                            |
| project_id _String_     | Project that the section resides in                                                                   |
| section_order _Integer_ | The order of the section. Defines the position of the section among all the sections in the project.  |
| order_key _String_      | Section's fractional-indexing order key: sections sort by comparing keys lexicographically within a project. May be `null` for projects not yet migrated. |
| is_collapsed _Boolean_  | Whether the section's tasks are collapsed (a `true` or `false` value).                                |
| is_deleted _Boolean_    | Whether the section is marked as deleted (a `true` or `false` value).                                 |
| is_archived _Boolean_   | Whether the section is marked as archived (a `true` or `false` value).                                |
| archived_at _String_    | The date when the section was archived (or `null` if not archived).                                   |
| added_at _String_       | The date when the section was created.                                                                |
| updated_at _String_     | The date when the section was updated.                                                                |

## Add a section

> Example add section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_add",
      "temp_id": "69ca86df-5ffe-4be4-9c3a-ad14fe98a58a",
      "uuid": "97b68ab2-f524-48da-8288-476a42cccf28",
      "args": {"name": "Groceries", "project_id": "9Bw8VQXxpwv56ZY2"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"97b68ab2-f524-48da-8288-476a42cccf28": "ok"},
  "temp_id_mapping": {"69ca86df-5ffe-4be4-9c3a-ad14fe98a58a": "6X7FxXvX84jHphx2"},
  ...
}
```

Add a new section to a project.

#### Command arguments

| Argument                | Required | Description                                                                                          |
|-------------------------|----------|------------------------------------------------------------------------------------------------------|
| name _String_           | Yes      | The name of the section.                                                                             |
| project_id _String_     | Yes      | The ID of the parent project.                                                                        |
| description _String_    | No       | The description of the section.                                                                      |
| section_order _Integer_ | No       | The order of the section. Defines the position of the section among all the sections in the project. |
| order_key _String_      | No       | Fractional-indexing order key for the new section. When omitted, the backend generates a key placing the section at the bottom of the project's list. If another section in the project already uses the requested key, the backend stores an adjusted key placing the section immediately after that position; the corrected value is returned via sync. |

## Update a section

> Example update section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_update",
      "uuid": "afda2f29-319c-4d09-8162-f2975bed3124",
      "args": {"id": "6X7FxXvX84jHphx2", "name": "Supermarket"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"afda2f29-319c-4d09-8162-f2975bed3124": "ok"},
  ...
}
```

Updates section attributes.

#### Command arguments

| Argument               | Required | Description                                                            |
|------------------------|----------|------------------------------------------------------------------------|
| id _String_            | Yes      | The ID of the section.                                                 |
| name _String_          | No       | The name of the section.                                               |
| description _String_   | No       | The description of the section. Pass `null` to clear it; omit to leave it unchanged. |
| order_key _String_     | No       | Fractional-indexing order key. When omitted, the current key is kept. If another section in the project already uses the requested key, the backend stores an adjusted key placing the section immediately after that position; the corrected value is returned via sync. |
| is_collapsed _Boolean_ | No       | Whether the section's tasks are collapsed (a `true` or `false` value). |

## Move a section

> Example move section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_move",
      "uuid": "a8583f66-5885-4729-9e55-462e72d685ff",
      "args": {"id": "6X7FxXvX84jHphx2", "project_id": "9Bw8VQXxpwv56ZY2"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"a8583f66-5885-4729-9e55-462e72d685ff": "ok"},
  ...
}
```

Move section to a different location.

#### Command arguments

| Argument            | Required | Description                    |
|---------------------|----------|--------------------------------|
| id _String_         | Yes      | The ID of the section.         |
| project_id _String_ | No       | ID of the destination project. |

## Reorder sections

> Example reorder sections request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_reorder",
      "uuid": "109af205-6ff7-47fa-a5f8-1f13e59744ef",
      "args": {
        "sections": [
          {"id": "6X7FxXvX84jHphx2", "section_order": 1},
          {"id": "6X9FxXvX64jjphx3", "section_order": 2}
        ]
      }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"109af205-6ff7-47fa-a5f8-1f13e59744ef": "ok"},
  ...
}
```

The command updates `section_order` properties of sections in bulk.

**Deprecation notice:** This command is deprecated and will be removed in a future
version. Reorder sections by setting the `order_key` property via `section_update`
instead: a reorder then updates a single section rather than rewriting the order of
every section in the project.

#### Command arguments

| Argument                    | Required | Description                                                                                                                           |
|-----------------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------|
| sections _Array of Objects_ | Yes      | An array of objects to update. Each object contains two attributes: `id` of the section to update and `section_order`, the new order. |

## Delete a section

> Example delete section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_delete",
      "uuid": "cebb5267-3e3c-40da-af44-500649281936",
      "args": {"id": "6X7FxXvX84jHphx2"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"cebb5267-3e3c-40da-af44-500649281936": "ok"},
  ...
}
```

Delete a section and all its child tasks.

#### Command arguments

| Argument    | Required | Description                  |
|-------------|----------|------------------------------|
| id _String_ | Yes      | ID of the section to delete. |

## Archive a section

> Example archive section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_archive",
      "uuid": "2451f267-46ab-4f0e-8db7-82a9cd576f72",
      "args": {"id": "6X7FxXvX84jHphx2"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"2451f267-46ab-4f0e-8db7-82a9cd576f72": "ok"},
  ...
}
```

Archive a section and all its child tasks.

#### Command arguments

| Argument    | Required | Description            |
|-------------|----------|------------------------|
| id _String_ | Yes      | Section ID to archive. |

## Unarchive a section

> Example unarchive section request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[{
      "type": "section_unarchive",
      "uuid": "cd3a4b4b-182e-4733-b6b5-20a621ba98b8",
      "args": {"id": "6X7FxXvX84jHphx2"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"cd3a4b4b-182e-4733-b6b5-20a621ba98b8": "ok"},
  ...
}
```

This command is used to unarchive a section.

#### Command arguments

| Argument    | Required | Description             |
|-------------|----------|-------------------------|
| id _String_ | Yes      | Section ID to unarchive |
