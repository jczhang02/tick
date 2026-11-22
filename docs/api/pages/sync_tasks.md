# Sync: Tasks

Task duration amounts must be positive integers.
The API accepts durations longer than 24 hours, but the Todoist web and desktop
date picker limits duration edits to 24 hours. Changing only a task's due date
in the picker preserves an existing longer duration.

> An example task object:

```
{
    "id": "6X7rM8997g3RQmvh",
    "user_id": "2671355",
    "project_id": "6Jf8VQXxpwv56VQ7",
    "content": "Buy Milk",
    "description": "",
    "priority": 1,
    "due": null,
    "deadline": null,
    "parent_id": null,
    "child_order": 1,
    "order_key": "a1V",
    "section_id": "3Ty8VQXxpwv28PK3",
    "day_order": -1,
    "is_collapsed": false,
    "labels": ["Food", "Shopping"],
    "added_by_uid": "2671355",
    "assigned_by_uid": "2671355",
    "responsible_uid": null,
    "checked": false,
    "is_deleted": false,
    "added_at": "2025-01-21T21:28:43.841504Z",
    "updated_at": "2025-01-21T21:28:43Z",
    "completed_at": null,
    "deadline": null,
    "duration": {
        "amount": 15,
        "unit": "minute"
    }
```

#### Properties

| Property                 | Description                                                                                                                                                                                                                                                                                                                            |
|--------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| id _String_              | The ID of the task.                                                                                                                                                                                                                                                                                                                    |
| user_id _String_         | The owner of the task.                                                                                                                                                                                                                                                                                                                 |
| project_id _String_      | The ID of the parent project.                                                                                                                                                                                                                                                                                                          |
| content _String_         | The text of the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center.                                                                         |
| description _String_     | A description for the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center.                                                                   |
| due _Object_             | The due date of the task. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.                                                                                                                                                                                                                                                |
| deadline _Object_        | The deadline of the task. See the [Deadlines](https://developer.todoist.com/api/v1/deadlines.md) section for more details.                                                                                                                                                                                                                                                |
| priority _Integer_       | The priority of the task (a number between `1` and `4`, `4` for very urgent and `1` for natural). <br>**Note**: Keep in mind that `very urgent` is the priority 1 on clients. So, `p1` will return `4` in the API.                                                                                                                     |
| parent_id _String_       | The ID of the parent task. Set to `null` for root tasks.                                                                                                                                                                                                                                                                               |
| child_order _Integer_    | The order of the task. Defines the position of the task among all the tasks with the same parent.                                                                                                                                                                                                                                      |
| order_key _String_       | Task's fractional-indexing order key: tasks sort by comparing keys lexicographically among siblings sharing the same project, section and parent task. May be `null` for tasks not yet migrated.                                                                                                                                     |
| section_id _String_      | The ID of the parent section. Set to `null` for tasks not belonging to a section.                                                                                                                                                                                                                                                      |
| day_order _Integer_      | The order of the task inside the `Today` or `Next 7 days` view (a number, where the smallest value would place the task at the top).                                                                                                                                                                                                   |
| is_collapsed _Boolean_   | Whether the task's sub-tasks are collapsed (a `true` or `false` value).                                                                                                                                                                                                                                                                |
| labels _Array of String_ | The task's labels (a list of names that may represent either personal or shared labels).                                                                                                                                                                                                                                               |
| added_by_uid _String_    | The ID of the user who created the task. This makes sense for shared projects only. For tasks created before 31 Oct 2019 the value is set to null. Cannot be set explicitly or changed via API.                                                                                                                                        |
| assigned_by_uid _String_ | The ID of the user who assigned the task. This makes sense for shared projects only. Accepts any user ID from the list of project collaborators. If this value is unset or invalid, it will automatically be set up to your uid.                                                                                                       |
| responsible_uid _String_ | The ID of user who is responsible for accomplishing the current task. This makes sense for shared projects only. Accepts any user ID from the list of project collaborators or `null` or an empty string to unset.                                                                                                                     |
| checked _Boolean_        | Whether the task is marked as completed (a `true` or `false` value).                                                                                                                                                                                                                                                                   |
| is_deleted _Boolean_     | Whether the task is marked as deleted (a `true` or `false` value).                                                                                                                                                                                                                                                                     |
| completed_at _String_    | The date when the task was completed (or `null` if not completed).                                                                                                                                                                                                                                                                     |
| added_at _String_        | The datetime when the task was created.                                                                                                                                                                                                                                                                                                |
| updated_at _String_      | The datetime when the task was updated.                                                                                                                                                                                                                                                                                                |
| completed_at _String_    | The datetime when the task was completed.                                                                                                                                                                                                                                                                                              |
| duration _Object_        | Object representing a task's duration. Includes a positive integer (greater than zero) for the `amount` of time the task will take, and the `unit` of time that the amount represents which must be either `minute` or `day`. Both the `amount` and `unit` **must** be defined. The object will be `null` if the task has no duration. |

## Add a task

> Example add task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_add",
        "temp_id": "43f7ed23-a038-46b5-b2c9-4abda9097ffa",
        "uuid": "997d4b43-55f1-48a9-9e66-de5785dfd69b",
        "args": {
            "content": "Buy Milk",
            "project_id": "6Jf8VQXxpwv56VQ7",
            "labels": ["Food", "Shopping"]
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"997d4b43-55f1-48a9-9e66-de5785dfd69b": "ok"},
  "temp_id_mapping": {"43f7ed23-a038-46b5-b2c9-4abda9097ffa": "6X7rM8997g3RQmvh"},
  ...
}
```

Add a new task to a project.

#### Command arguments

| Argument                    | Required | Description                                                                                                                                                                                                                                                          |
|-----------------------------|----------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| content _String_            | Yes      | The text of the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center.       |
| description _String_        | No       | A description for the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center. |
| project_id _String_         | No       | The ID of the project to add the task to (a number or a temp id). By default the task is added to the user’s `Inbox` project.                                                                                                                                        |
| due _Object_                | No       | The due date of the task. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.                                                                                                                                                                              |
| deadline _Object_           | No       | The deadline of the task. See the [Deadlines](https://developer.todoist.com/api/v1/deadlines.md) section for more details.                                                                                                                                                                              |
| priority _Integer_          | No       | The priority of the task (a number between `1` and `4`, `4` for very urgent and `1` for natural). <br>**Note**: Keep in mind that `very urgent` is the priority 1 on clients. So, `p1` will return `4` in the API.                                                   |
| parent_id _String_          | No       | The ID of the parent task. Set to `null` for root tasks.                                                                                                                                                                                                             |
| child_order _Integer_       | No       | The order of task. Defines the position of the task among all the tasks with the same parent.                                                                                                                                                                        |
| order_key _String_          | No       | Fractional-indexing order key for the new task. When omitted, the backend generates a key placing the task at the bottom of the list of siblings sharing the same project, section and parent task. If a sibling already uses the requested key, the backend stores an adjusted key placing the task immediately after that position; the corrected value is returned via sync. |
| section_id _String_         | No       | The ID of the section. Set to `null` for tasks not belonging to a section.                                                                                                                                                                                           |
| day_order _Integer_         | No       | The order of the task inside the `Today` or `Next 7 days` view. Must be a signed 32-bit integer between `-2147483648` and `2147483647`; smaller values place the task nearer the top.                                                                                   |
| is_collapsed _Boolean_      | No       | Whether the task's sub-tasks are collapsed (a `true` or `false` value).                                                                                                                                                                                              |
| labels _Array of String_    | No       | The task's labels (a list of names that may represent either personal or shared labels).                                                                                                                                                                             |
| assigned_by_uid _String_    | No       | The ID of user who assigns the current task. This makes sense for shared projects only. Accepts `0` or any user ID from the list of project collaborators. If this value is unset or invalid, it will be automatically setup to your uid.                            |
| responsible_uid _String_    | No       | The ID of user who is responsible for accomplishing the current task. This makes sense for shared projects only. Accepts any user ID from the list of project collaborators or `null` or an empty string to unset.                                                   |
| auto_reminder _Boolean_     | No       | When this option is enabled, the default reminder will be added to the new item if it has a due date with time set. See also the [auto_reminder user option](https://developer.todoist.com/api/v1/sync/user.md) for more info about the default reminder.                                               |
| auto_parse_labels _Boolean_ | No       | When this option is enabled, the labels will be parsed from the task content and added to the task. In case the label doesn't exist, a new one will be created.                                                                                                      |
| duration _Object_           | No       | The task's duration. Includes a positive integer (greater than zero) for the `amount` of time the task will take, and the `unit` of time that the amount represents which must be either `minute` or `day`. Both the `amount` and `unit` **must** be defined.        |

## Update a task

> Example update task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_update",
        "uuid": "aca17834-da6f-4605-bde0-bd10be228878",
        "args": {
            "id": "6X7rM8997g3RQmvh",
            "content": "Buy Coffee",
            "due": {"string": "tomorrow at 10:00" }
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"318d16a7-0c88-46e0-9eb5-cde6c72477c8": "ok"},
  ...
}
```

Updates task attributes. Please note that updating the parent, moving,
completing or uncompleting tasks is not supported by `item_update`, more
specific commands have to be used instead.

#### Command arguments

| Argument                 | Required | Description                                                                                                                                                                                                                                                                                                                 |
|--------------------------|----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| id _String_              | Yes      | The ID of the task.                                                                                                                                                                                                                                                                                                         |
| content _String_         | No       | The text of the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center.                                                              |
| description _String_     | No       | A description for the task. This value may contain markdown-formatted text and hyperlinks. Details on markdown support can be found in the [Text Formatting article](https://www.todoist.com/help/articles/format-text-in-a-todoist-task-e5dHw9) in the Help Center.                                                        |
| due _Object_             | No       | The due date of the task. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.                                                                                                                                                                                                                                     |
| deadline _Object_        | No       | The deadline of the task. See the [Deadlines](https://developer.todoist.com/api/v1/deadlines.md) section for more details.                                                                                                                                                                                                                                     |
| priority _Integer_       | No       | The priority of the task (a number between `1` and `4`, `4` for very urgent and `1` for natural). <br>**Note**: Keep in mind that `very urgent` is the priority 1 on clients. So, `p1` will return `4` in the API.                                                                                                          |
| is_collapsed _Boolean_   | No       | Whether the task's sub-tasks are collapsed (a `true` or `false` value).                                                                                                                                                                                                                                                     |
| labels _Array of String_ | No       | The task's labels (a list of names that may represent either personal or shared labels).                                                                                                                                                                                                                                    |
| assigned_by_uid _String_ | No       | The ID of the user who assigned the task. This makes sense for shared projects only. Accepts `0` or any user ID from the list of project collaborators. If this value is unset or invalid, it will be automatically setup to your uid.                                                                                      |
| responsible_uid _String_ | No       | The ID of the user who is responsible for accomplishing the task. This makes sense for shared projects only. Accepts any user ID from the list of project collaborators or `null` or an empty string to unset.                                                                                                              |
| day_order _Integer_      | No       | The order of the task inside the `Today` or `Next 7 days` view. Must be a signed 32-bit integer between `-2147483648` and `2147483647`; smaller values place the task nearer the top.                                                                                                                                          |
| duration _Object_        | No       | The task's duration. Must a positive integer (greater than zero) for the `amount` of time the task will take, and the `unit` of time that the amount represents which must be either `minute` or `day`. Both the `amount` and `unit` **must** be defined. The object should be set to `null` to remove the task's duration. |
| order_key _String_       | No       | Fractional-indexing order key. When omitted, the current key is kept. If a sibling already uses the requested key, the backend stores an adjusted key placing the task immediately after that position; the corrected value is returned via sync.                                                                       |

## Move a task

> Example move task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_move",
        "uuid": "318d16a7-0c88-46e0-9eb5-cde6c72477c8",
        "args": {
            "id": "6X7rM8997g3RQmvh",
            "parent_id": "6X7rf9x6pv2FGghW"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"318d16a7-0c88-46e0-9eb5-cde6c72477c8": "ok"},
  ...
}
```

Move task to a different location. Only one of `parent_id`, `section_id` or
`project_id` must be set.

#### Command arguments

| Argument            | Required | Description                                                                                 |
|---------------------|----------|---------------------------------------------------------------------------------------------|
| id _String_         | Yes      | The ID of the task.                                                                         |
| parent_id _String_  | No       | ID of the destination parent task. The task becomes the last child task of the parent task. |
| section_id _String_ | No       | ID of the destination section. The task becomes the last root task of the section.          |
| project_id _String_ | No       | ID of the destination project. The task becomes the last root task of the project.          |

## Reorder tasks

> Example reorder tasks request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands=[
    {
        "type": "item_reorder",
        "uuid": "bf0855a3-0138-4b76-b895-88cad8db9edc",
        "args": {
            "items": [
                {"id": "6X7rM8997g3RQmvh", "child_order": 1},
                {"id": "6X7rfFVPjhvv84XG", "child_order": 2}
            ]
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"bf0855a3-0138-4b76-b895-88cad8db9edc": "ok"},
  ...
}
```

The command updates `child_order` properties of items in bulk.

**Deprecation notice:** This command is deprecated and will be removed in a future
version. Reorder tasks by setting the `order_key` property via `item_update`
instead: a reorder then updates a single task rather than rewriting the order of
every task in the project.

#### Command arguments

| Argument                 | Required | Description                                                                                                                      |
|--------------------------|----------|----------------------------------------------------------------------------------------------------------------------------------|
| items _Array of Objects_ | Yes      | An array of objects to update. Each object contains two attributes: `id` of the item to update and `child_order`, the new order. |

## Delete tasks

> Example delete task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_delete",
        "uuid": "f8539c77-7fd7-4846-afad-3b201f0be8a5",
        "args": {"id": "6X7rfFVPjhvv84XG"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"f8539c77-7fd7-4846-afad-3b201f0be8a5": "ok"},
  ...
}
```

Delete a task and all its sub-tasks.

#### Command arguments

| Argument    | Required | Description               |
|-------------|----------|---------------------------|
| id _String_ | Yes      | ID of the task to delete. |

## Complete task

> Example complete task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_complete",
        "uuid": "a74bfb5c-5f1d-4d14-baea-b7415446a871",
        "args": {
            "id": "6X7rfFVPjhvv84XG",
            "date_completed": "2017-01-02T01:00:00.000000Z"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"a74bfb5c-5f1d-4d14-baea-b7415446a871": "ok"},
  ...
}
```

Completes a task and its sub-tasks and moves them to the archive. See also `item_close` for a
simplified version of the command.

#### Command arguments

| Argument              | Required | Description                                                                                                                    |
|-----------------------|----------|--------------------------------------------------------------------------------------------------------------------------------|
| id _String_           | Yes      | Task ID to complete.                                                                                                           |
| date_completed _Date_ | No       | RFC3339-formatted date of completion of the task (in UTC). If not set, or more than 24 hours in the future, the server will set the value to the current timestamp. |
| from_undo _Boolean_   | No       | If `true`, skips incrementing completion stats. Used when restoring task state after undoing a completion.                     |

## Uncomplete item

> Example uncomplete task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_uncomplete",
        "uuid": "710a60e1-174a-4313-bb9f-4df01e0349fd",
        "args": {"id": "2995104339"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"710a60e1-174a-4313-bb9f-4df01e0349fd": "ok"},
  ...
}
```

This command is used to uncomplete and restore an completed task.

Any ancestor items or sections will also be reinstated. Items will have the `checked` value reset.

The reinstated items and sections will appear at the end of the list within their parent, after any previously
active tasks.

#### Command arguments

| Argument    | Required | Description           |
|-------------|----------|-----------------------|
| id _String_ | Yes      | Task ID to uncomplete |

## Complete a recurring task

> Example complete recurring task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_update_date_complete",
        "uuid": "c5888360-96b1-46be-aaac-b49b1135feab",
        "args": {
            "id": "2995104339",
            "due": {"date": "2014-10-30", "string": "every day"},
            "is_forward": 1,
            "reset_subtasks": 0
        }
    }]
```

> Example response:

```shell
{
  ...
  "sync_status": {"c5888360-96b1-46be-aaac-b49b1135feab": "ok"},
  ...
}
```

Complete a recurring task. The reason why this is a special case is because
we need to mark a recurring completion (and using `item_update` won't do
this). See also `item_close` for a simplified version of the command.

#### Command arguments

| Argument                 | Required | Description                                                                                                                                                                                        |
|--------------------------|----------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| id _String_              | Yes      | The ID of the item to update (a number or a temp id).                                                                                                                                              |
| due _Object_             | No       | The due date of the task. See the [Due dates](https://developer.todoist.com/api/v1/due-dates.md) section for more details.                                                                                                            |
| is_forward _Boolean_     | No       | Set this argument to 1 for completion, or 0 for uncompletion (e.g., via undo). By default, this argument is set to 1 (completion).                                                                 |
| reset_subtasks _Boolean_ | No       | Set this property to 1 to reset subtasks when a recurring task is completed. By default, this property is not set (0), and subtasks will retain their existing status when the parent task recurs. |

## Close task

> Example close task request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_close",
        "uuid": "c5888360-96b1-46be-aaac-b49b1135feab",
        "args": {"id": "2995104339"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"c5888360-96b1-46be-aaac-b49b1135feab": "ok"},
  ...
}
```

A simplified version of `item_complete` / `item_update_date_complete`. The command
does exactly what official clients do when you close a task: regular tasks are
completed and moved to the archive, recurring tasks are scheduled to their next occurrence.

#### Command arguments

| Argument    | Required | Description                                          |
|-------------|----------|------------------------------------------------------|
| id _String_ | Yes      | The ID of the item to close (a number or a temp id). |

## Update day orders

> Example update day orders request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "item_update_day_orders",
        "uuid": "dbeb40fc-905f-4d8a-8bae-547d3bbd6e91",
        "args": {"ids_to_orders": {"2995104339": 1}}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"dbeb40fc-905f-4d8a-8bae-547d3bbd6e91": "ok"},
  ...
}
```

Update the day orders of multiple tasks at once.

**Deprecation notice:** This command is deprecated and will be removed in a future
version. Place a task in the Today/Upcoming order by setting `order_key` via
`user_item_order_update` instead: a reorder then updates a single task rather than
resending the order of every task in the list, and the key is stable, so two
clients reordering at once no longer overwrite each other's positions.

#### Command arguments

| Argument               | Required | Description                                                                                  |
|------------------------|----------|----------------------------------------------------------------------------------------------|
| ids_to_orders _Object_ | Yes      | A mapping where each key is a task ID and each value is a `day_order` (a signed 32-bit integer between `-2147483648` and `2147483647`): `item_id: day_order`. |
