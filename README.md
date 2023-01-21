# [rbr-sync] [![ci-badge][]][ci]

RBR Sync helps you to organize [RallySimFans] favorite stages.
It fetches stages from your Notion DB.
Filters them according to the selected tags.
And finnaly writes selected stage IDs to the RSF favorites file.

![demo]

[rbr-sync]:     https://github.com/2m/rbr-sync
[ci-badge]:     https://github.com/2m/rbr-sync/actions/workflows/ci.yaml/badge.svg
[ci]:           https://github.com/2m/rbr-sync/actions/workflows/ci.yaml
[RallySimFans]: https://www.rallysimfans.hu/rbr/index.php
[demo]:         docs/demo.gif

## Usage

1. Create a new [Notion Integration]

1. It only needs `Read content` capability

    ![capabilities]

1. Copy `Internal Integration Token`

    ![token]

1. Create a Notion database

1. The database must have columns with these exact names and types:
    * `Name` - title type
    * `Tags` - multiselect
    * `ID` - numeric

    ![database]

1. Add your new integration to your Notion database from the sandwitch menu

    ![connections]

1. Extract database ID. For example, the following DB block link

        https://dvim.notion.site/ed8fad327420410da7c24ad18e73d7ef?v=bfded56a8e00468a9db473397e91e33c

   points to the database with ID `ed8fad327420410da7c24ad18e73d7ef`

1. Use the `Token` and `DB ID` in the app to fetch stages and their tags

    ![fetching]

1. Select tags and click `Write` to update the `favorites.ini` file

[Notion Integration]: https://www.notion.so/my-integrations
[database]: docs/database.png
[capabilities]: docs/capabilities.png
[token]: docs/token.png
[connections]: docs/connections.png
[fetching]: docs/fetching.gif
