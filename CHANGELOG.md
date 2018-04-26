# Changelog

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Changed
- User Stats accounting now collects the time seeded
- Usernames may now only contain letters, numbers, _ and -. And they must begin with a letter and have to be at least 4 characters long.
- Passwords must now be at least 8 characters long.

### Fixed
- Uploaded torrents without a specific name, now have the `.torrent` extension removed.
- Custom File Input fields now set the name of the selected file as label.
- Fixed wrong accounting for uploaded data, due to a typo.
- Downloaded torrents now have the `.torrent` suffix appended.

### Removed
- bip_bencode in favour for serde_bencode.

## [0.1.0]

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
