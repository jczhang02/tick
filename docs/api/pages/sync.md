# Sync

The Todoist Sync endpoint is specially designed for efficient data sync between
clients (e.g. our mobile apps) and Todoist.

Sync requests should be made in HTTP POST (`application/x-www-form-urlencoded`).
Sync responses, including errors, will be returned in JSON.

The Sync endpoint supports the following features:

- [Batching](https://developer.todoist.com/api/v1/sync/overview.md): reading and writing of
  multiple resources can be done in a single HTTP request. Batch requests help
  clients reduce the number of network calls needed to sync resources.
- [Incremental sync](https://developer.todoist.com/api/v1/sync/overview.md): You only retrieve
  data that is updated since the last time you performed a sync request.

_Refer to [Request Limits](https://developer.todoist.com/api/v1/request-limits.md) to learn more about the number of requests/commands
you have for the Sync API_

## Sections

- [Overview](https://developer.todoist.com/api/v1/sync/overview.md)
- [Workspace](https://developer.todoist.com/api/v1/sync/workspace.md)
- [Workspace users](https://developer.todoist.com/api/v1/sync/workspace-users.md)
- [View Options](https://developer.todoist.com/api/v1/sync/view-options.md)
- [Project View Options Defaults](https://developer.todoist.com/api/v1/sync/project-view-options-defaults.md)
- [User](https://developer.todoist.com/api/v1/sync/user.md)
- [Sharing](https://developer.todoist.com/api/v1/sync/sharing.md)
- [Sections](https://developer.todoist.com/api/v1/sync/sections.md)
- [Reminders](https://developer.todoist.com/api/v1/sync/reminders.md)
- [Projects](https://developer.todoist.com/api/v1/sync/projects.md)
- [Folders](https://developer.todoist.com/api/v1/sync/folders.md)
- [Comments](https://developer.todoist.com/api/v1/sync/comments.md)
- [Live notifications](https://developer.todoist.com/api/v1/sync/live-notifications.md)
- [Labels](https://developer.todoist.com/api/v1/sync/labels.md)
- [Tasks](https://developer.todoist.com/api/v1/sync/tasks.md)
- [User item orders](https://developer.todoist.com/api/v1/sync/user-item-orders.md)
- [Filters](https://developer.todoist.com/api/v1/sync/filters.md)
- [Workspace Filters](https://developer.todoist.com/api/v1/sync/workspace-filters.md)
