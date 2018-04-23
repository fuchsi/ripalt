# Changelog

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Cleanup thread to remove orphaned peers.
- Identity Provider for the API, which uses either the Session ID or a JWT.
    - New Setting: `jwt_secret`, the secret key for the JWTs.
- API Endpoint to get the own stats (/api/v1/user/stats)

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
