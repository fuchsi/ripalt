# Changelog

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Use the NFO as description if the description is empty
- Message API
    - `GET /api/v1/message/messages` get all messages in a folder.
    - `GET /api/v1/message/unread` get all unread messages in a folder.
    - `GET /api/v1/message/read` get / read a message.
    - `POST /api/v1/message/mark_read` mark messages as read.
    - `POST /api/v1/message/delete` delete messages.
    - `POST /api/v1/message/send` send a new message.
- Message Frontend
- FAQ / Rules and other _static_ content.
- User Settings and Profile
- Torrent Comment API
    - `GET /api/v1/comment/torrent` get all comments for a torrent.
    - `GET /api/v/comment/get` get a single comment.
    - `POST /api/v1/comment/new` publish a new comment.
    - `POST /api/v1/comment/edit` edit a comment.
    - `POST /api/v1/comment/delete` delete a comment.
- Torrent Comment Frontend


### Changed
- `Template::render()` now returns `HttpResponse` instead of `Template`
- The `format_date` Helper now appends 'UTC' if no specific timezone is provided.

## [0.2.0] - 2018-04-30

### Added
- Cleanup thread to remove orphaned peers.
- Identity Provider for the API, which uses either the Session ID or a JWT.
    - New Setting: `jwt_secret`, the secret key for the JWTs.
- API endpoint to get the own stats (/api/v1/user/stats)
- User Profiles.
- Download NFOs.
- Edit and Delete Torrents.
- Support for user defined timezone and torrents per page settings.
- Own Identity Middleware, more or less a copy of the original one.
- API endpoints for the chat:
    - `GET /api/v1/chat/messages` to fetch messages.
    - `POST /api/v1/chat/publish` to publish a new message.
- A simple Chat (Shoutbox) with 2 Chatrooms (public and team)
- Show active(last active within 30m) users on the index page.
- Add Images to Torrent uploads.

### Changed
- User Stats accounting now collects the time seeded
- Usernames may now only contain letters, numbers, _ and -. And they must begin with a letter and have to be at least 4 characters long.
- Passwords must now be at least 8 characters long.
- Assets (for now just the stylesheets) are now generated at the build process and no longer shipped precompiled.
    - A seperate compile script, or something, may be added later.

### Fixed
- Uploaded torrents without a specific name, now have the `.torrent` extension removed.
- Custom File Input fields now set the name of the selected file as label.
- Fixed wrong accounting for uploaded data, due to a typo.
- Downloaded torrents now have the `.torrent` suffix appended.

### Removed
- bip_bencode in favour for serde_bencode.

## [0.1.0] - 2018-04-20

### Added

- Web Portal: minimal functionality
    - User Sign Up and Sign In
    - Upload Torrents
    - Browse / Search Torrents
    - View Torrent details
    - Download Torrents
    - A minimal ACL system
- Tracker
    - Tracking Torrents (/tracker/announce/...) with user/torrent stats accounting
    - Scraping
