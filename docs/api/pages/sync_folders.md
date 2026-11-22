# Sync: Folders

Folders group workspace projects together for display in the Project view. They are
only available for workspace (team) projects.

## Add a folder

> Example add folder request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "folder_add",
        "temp_id": "4ff1e388-5ca6-453a-b0e8-662ebf373b6b",
        "uuid": "32774db9-a1da-4550-8d9d-910372124fa4",
        "args": {
            "name": "Marketing",
            "workspace_id": "6X6WMMqgq2PWxjCX"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"32774db9-a1da-4550-8d9d-910372124fa4": "ok"},
  "temp_id_mapping": {"4ff1e388-5ca6-453a-b0e8-662ebf373b6b": "6X8jVQXxpwv56VQ9"},
  ...
}
```

Add a new folder to a workspace.

#### Command arguments

| Argument                          | Required | Description                                                                                                                                                                                                                                                     |
|------------------------------------|----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| name _String_                      | Yes      | The name of the folder.                                                                                                                                                                                                                                      |
| workspace_id _String_              | Yes      | Real or temp ID of the workspace the folder belongs to.                                                                                                                                                                                                      |
| add_project_ids _Array of String_  | No       | IDs of workspace projects to move into the folder as part of creating it.                                                                                                                                                                                    |
| default_order_key _String_         | No       | Fractional-indexing key for the workspace's shared default-ordering scope (workspace projects and folders together). When omitted, the backend generates a key placing the folder at the bottom of that scope. If another project or folder in the workspace already uses the requested key, the backend stores an adjusted key placing the folder immediately after that position; the corrected value is returned via sync. While clients are migrating, the legacy `default_order` integers of the workspace's projects and folders are renumbered to follow the key order, so clients that still sort by `default_order` see the same order. |
| default_order _Integer_            | No       | Legacy integer position within the workspace's shared default-ordering scope. Superseded by `default_order_key`; when both are omitted the folder is placed at the bottom of the scope.                                                                    |
| child_order _Integer_              | No       | The user-specific order of the folder, independent of the workspace's shared default ordering.                                                                                                                                                              |

## Update a folder

> Example update folder request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "folder_update",
        "uuid": "1ca42128-d12f-4a66-8413-4d6ff2838fde",
        "args": {
            "id": "6X8jVQXxpwv56VQ9",
            "workspace_id": "6X6WMMqgq2PWxjCX",
            "name": "Marketing & Growth"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"1ca42128-d12f-4a66-8413-4d6ff2838fde": "ok"},
  ...
}
```

Update an existing folder, and optionally move workspace projects in or out of it.

#### Command arguments

| Argument                             | Required | Description                                                                                                                                                                                                                                                  |
|----------------------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| id _String_                            | Yes      | The ID of the folder (could be a temp id).                                                                                                                                                                                                              |
| workspace_id _String_                  | Yes      | Real or temp ID of the workspace the folder belongs to.                                                                                                                                                                                                 |
| name _String_                          | No       | The name of the folder.                                                                                                                                                                                                                                 |
| default_order_key _String_             | No       | Fractional-indexing key for the workspace's shared default-ordering scope (workspace projects and folders together). When omitted, the current key is kept. If another project or folder in the workspace already uses the requested key, the backend stores an adjusted key placing the folder immediately after that position; the corrected value is returned via sync. While clients are migrating, the legacy `default_order` integers of the workspace's projects and folders are renumbered to follow the key order, so clients that still sort by `default_order` see the same order. When a workspace admin changes a key, the workspace's admin sorting is also marked as applied (`admin_sorting_applied`). |
| add_project_ids _Array of String_      | No       | IDs of workspace projects to move into the folder.                                                                                                                                                                                                      |
| remove_project_ids _Array of String_   | No       | IDs of workspace projects to move out of the folder.                                                                                                                                                                                                    |
