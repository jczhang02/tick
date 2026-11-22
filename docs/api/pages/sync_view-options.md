# Sync: View Options

> An example view option object:

```
{
    "view_type": "project",
    "object_id": "6Jf8VQXxpwv56VQ7",
    "filtered_by": "!assigned",
    "grouped_by": "priority",
    "sorted_by": "added_date",
    "sort_order": "asc",
    "show_completed_tasks": false,
    "view_mode": "calendar",
    "calendar_settings": { "layout": "WEEK", "visible_day_count": 3, "color": "PROJECT" },
    "is_deleted": false,
    "deadline": "no deadline"
}
```

#### Properties

| Property                       | Description                                                                                                                                                                                                           |
|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| view_type _Enum_               | The type of a view customization. `today` for the today view, `upcoming` for the upcoming view, `assigned_to_me` for the assigned to me view, `project` for a project, `label` for a label, `filter` for a personal filter or `workspace_filter` for a team filter. |
| object_id _String_             | The ID of the object referred to by `view_type`, when `view_type` is `project`, `label`, `filter` or `workspace_filter`.                                                                                               |
| filtered_by _String_           | A search query for this view customization. [Examples of searches](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) can be found in the Todoist help page.                                       |
| grouped_by _Enum_              | Grouping criteria for this view customization. One of `assignee`, `added_date`, `due_date`, `deadline`, `label`, `priority`, `project`, or `workspace`.                                                               |
| sorted_by _Enum_               | Sorting criteria for this view customization. One of `alphabetically`, `assignee`, `added_date`, `due_date`, `deadline`, `label`, `priority`, `project`, `workspace`, or `manual`.                                    |
| sort_order _Enum_              | Sorting order for this view customization. `asc` for ascending, `desc` for descending.                                                                                                                                |
| show_completed_tasks _Boolean_ | Whether completed tasks should be shown automatically in this view customization.                                                                                                                                     |
| view_mode _Enum_               | The mode in which to render tasks in this view customization. One of `list`, `board`, `calendar`, or `table`. **Note: This setting is ignored in projects, where `project.view_style` is used instead.**                       |
| deadline _String_              | A search query for this view customization. [Examples of deadline searches](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) can be found in the Todoist help page.                              |
| calendar_settings _JSON_       | The settings for the calendar when `view_mode` is set to `calendar`. `layout` is `WEEK` or `MONTH`; optional `visible_day_count` is `1`, `3`, or `7` for the week layout; optional `color` is `PRIORITY`, `LABEL`, or `PROJECT` and defaults to `PRIORITY`.                                               |
| property_settings _JSON_       | How the view shows task properties, such as `due` or `labels`, in every layout, holding only what the user changed: `order` lists property IDs in display order; `properties` maps a property ID to `is_hidden` and `width` (in pixels, where a layout shows the property in a column). Property IDs are opaque strings, and keys and attributes the API doesn't know are kept, so clients can add properties, like custom fields. |
| is_deleted _Boolean_           | Whether the view option is marked as deleted.                                                                                                                                                                         |

**Note:** `view_options.view_mode` is secondary to [`project.view_style`](https://developer.todoist.com/api/v1/sync/view-options.md) for projects in Todoist clients. The former is set per user, while the latter is set per project.

## Set a view option

| Argument                       | Required | Description                                                                                                                                                                                                                   |
|--------------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| view_type _Enum_               | Yes      | Type of the view customization to be set. `today` for the today view, `upcoming` for the upcoming view, `assigned_to_me` for the assigned to me view, `project` for a project, `label` for a label, `filter` for a personal filter or `workspace_filter` for a team filter. |
| object_id _String_             | No       | ID of the object referred to by `view_type`, required when `view_type` is `project`, `label`, `filter` or `workspace_filter`.                                                                                                  |
| filtered_by _String_           | No       | Search query. [Examples of searches](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) can be found in the Todoist help page.                                                                             |
| grouped_by _Enum_              | No       | Grouping criteria. One of `assignee`, `added_date`, `due_date`, `deadline`, `label`, `priority`, `project`, or `workspace`.                                                                                                   |
| sorted_by _Enum_               | No       | Sorting criteria. One of `alphabetically`, `assignee`, `added_date`, `due_date`, `deadline`, `label`, `priority`, `project`, `workspace`, or `manual`.                                                                        |
| sort_order _Enum_              | No       | Sorting order. `asc` for ascending, `desc` for descending.                                                                                                                                                                    |
| show_completed_tasks _Boolean_ | No       | Whether completed tasks should be shown automatically in this view customization.                                                                                                                                             |
| view_mode _Enum_               | No       | The mode in which to render tasks. One of `list`, `board`, `calendar`, or `table`.                                                                                                                                                     |
| deadline _String_              | No       | A search query for this view customization. [Examples of deadline searches](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) can be found in the Todoist help page.                                      |
| calendar_settings _JSON_       | No       | The settings for the calendar when `view_mode` is set to `calendar`. `layout` is `WEEK` or `MONTH`; optional `visible_day_count` is `1`, `3`, or `7` for the week layout; optional `color` is `PRIORITY`, `LABEL`, or `PROJECT` and defaults to `PRIORITY`.                                                       |
| property_settings _JSON_       | No       | The property settings (see above), as a patch: objects merge into the saved ones, arrays such as `order` replace them, and `null` removes its path (`{"properties": {"due": {"width": null}}}` resets one width); `{}` changes nothing, and `property_settings: null` clears them all. Settings that are invalid, or over 32 KiB once merged, are ignored. |

> Example set view option request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "view_options_set",
        "uuid": "997d4b43-55f1-48a9-9e66-de5785dfd696",
        "args": {
            "view_type": "project",
            "object_id": "6Jf8VQXxpwv56VQ7",
            "view_mode": "board",
            "grouped_by": "assignee"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"997d4b43-55f1-48a9-9e66-de5785dfd696": "ok"},
  ...
}
```

## Delete view option

| Argument           | Required | Description                                                                                                                                                                     |
|--------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| view_type _Enum_   | Yes      | Type of the view customization to delete. `today` for the today view, `upcoming` for the upcoming view, `assigned_to_me` for the assigned to me view, `project` for a project, `label` for a label, or `filter` for a filter. |
| object_id _String_ | No       | ID of the object referred to by `view_type`, required when `view_type` is `project`, `label`, `filter` or `workspace_filter`.                                                    |

> Example delete view option request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "view_options_delete",
        "uuid": "f8539c77-7fd7-4846-afad-3b201f0be8a6",
        "args": {
            "view_type": "today"
        }
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"f8539c77-7fd7-4846-afad-3b201f0be8a6": "ok"},
  ...
}
```
