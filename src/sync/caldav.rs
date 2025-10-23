//! CalDAV client implementation
//!
//! Handles communication with CalDAV servers using HTTP/WebDAV protocols.

use reqwest::Client;
use url::Url;
use crate::Result;

/// CalDAV client for syncing calendar data
pub struct CalDavClient {
    /// HTTP client for making requests
    client: Client,
    /// Server base URL
    server_url: Url,
    /// Username for authentication
    username: String,
    /// Password for authentication
    password: String,
}

/// Represents a CalDAV calendar collection
#[derive(Debug, Clone)]
pub struct CalendarInfo {
    /// Calendar display name
    pub name: String,
    /// Calendar URL
    pub url: String,
    /// Calendar color (optional)
    pub color: Option<String>,
}

/// Represents a calendar event from CalDAV
#[derive(Debug, Clone)]
pub struct CalDavEvent {
    /// Event UID
    pub uid: String,
    /// Event URL (for fetching full data)
    pub url: String,
    /// ETag for change detection
    pub etag: String,
}

impl CalDavClient {
    /// Create a new CalDAV client
    ///
    /// # Arguments
    /// * `server_url` - The CalDAV server URL
    /// * `username` - Username for authentication
    /// * `password` - Password for authentication
    ///
    /// # Design Decision
    /// We use reqwest for HTTP because:
    /// - It's the de facto standard HTTP client for Rust
    /// - Supports async/await natively
    /// - Has built-in basic auth support
    /// - Handles TLS well
    pub fn new(server_url: String, username: String, password: String) -> Result<Self> {
        let server_url = Url::parse(&server_url)
            .map_err(|e| anyhow::anyhow!("Invalid server URL: {}", e))?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            server_url,
            username,
            password,
        })
    }

    /// Discover calendar home URL using well-known discovery
    ///
    /// # Returns
    /// The calendar home URL for the authenticated user
    ///
    /// # Design Decision
    /// We use the .well-known/caldav discovery mechanism (RFC 6764) because:
    /// - It's the standard way to discover CalDAV services
    /// - Allows users to just provide a domain name
    /// - Most CalDAV servers support it (including FastMail)
    pub async fn discover_calendar_home(&self) -> Result<String> {
        let url = self.server_url.join("/.well-known/caldav")?;

        let body = r#"<?xml version="1.0" encoding="utf-8" ?>
<propfind xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <prop>
    <C:calendar-home-set/>
  </prop>
</propfind>"#;

        let response = self.client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "0")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Calendar discovery failed: {}", response.status()));
        }

        let xml = response.text().await?;
        parse_calendar_home(&xml)
    }

    /// List all calendars at the calendar home URL
    ///
    /// # Arguments
    /// * `calendar_home_url` - The calendar home URL from discovery
    ///
    /// # Returns
    /// A list of available calendars
    pub async fn list_calendars(&self, calendar_home_url: &str) -> Result<Vec<CalendarInfo>> {
        let url = self.server_url.join(calendar_home_url)?;

        let body = r#"<?xml version="1.0" encoding="utf-8" ?>
<propfind xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">
  <prop>
    <resourcetype/>
    <displayname/>
    <CS:calendar-color/>
  </prop>
</propfind>"#;

        let response = self.client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("List calendars failed: {}", response.status()));
        }

        let xml = response.text().await?;
        parse_calendars(&xml)
    }

    /// List events in a calendar
    ///
    /// # Arguments
    /// * `calendar_url` - The calendar collection URL
    ///
    /// # Returns
    /// A list of events (with UIDs, URLs, and ETags)
    ///
    /// # Design Decision
    /// We use the calendar-query REPORT method instead of PROPFIND because:
    /// - It's more efficient (can filter by date range if needed)
    /// - Returns UIDs directly
    /// - Standard CalDAV method for querying events
    pub async fn list_events(&self, calendar_url: &str) -> Result<Vec<CalDavEvent>> {
        let url = self.server_url.join(calendar_url)?;

        let body = r#"<?xml version="1.0" encoding="utf-8" ?>
<calendar-query xmlns="urn:ietf:params:xml:ns:caldav" xmlns:D="DAV:">
  <D:prop>
    <D:getetag/>
    <calendar-data>
      <comp name="VCALENDAR">
        <comp name="VEVENT">
          <prop name="UID"/>
        </comp>
      </comp>
    </calendar-data>
  </D:prop>
  <filter>
    <comp-filter name="VCALENDAR">
      <comp-filter name="VEVENT"/>
    </comp-filter>
  </filter>
</calendar-query>"#;

        let response = self.client
            .request(reqwest::Method::from_bytes(b"REPORT")?, url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("List events failed: {}", response.status()));
        }

        let xml = response.text().await?;
        parse_events(&xml)
    }

    /// Download the full iCalendar data for an event
    ///
    /// # Arguments
    /// * `event_url` - The event resource URL
    ///
    /// # Returns
    /// The raw iCalendar data (VEVENT component)
    pub async fn get_event(&self, event_url: &str) -> Result<String> {
        let url = self.server_url.join(event_url)?;

        let response = self.client
            .get(url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Get event failed: {}", response.status()));
        }

        Ok(response.text().await?)
    }

    /// Get the sync token for a calendar (for efficient syncing)
    ///
    /// # Arguments
    /// * `calendar_url` - The calendar collection URL
    ///
    /// # Returns
    /// The current sync token, if supported
    pub async fn get_sync_token(&self, _calendar_url: &str) -> Result<Option<String>> {
        // TODO: Perform PROPFIND to get sync-token property
        // TODO: Return None if sync-token is not supported
        todo!("Implement get_sync_token")
    }

    /// Perform a sync-collection report to get changes since last sync
    ///
    /// # Arguments
    /// * `calendar_url` - The calendar collection URL
    /// * `sync_token` - The previous sync token
    ///
    /// # Returns
    /// A tuple of (new_events, deleted_event_urls, new_sync_token)
    pub async fn sync_collection(
        &self,
        _calendar_url: &str,
        _sync_token: &str,
    ) -> Result<(Vec<CalDavEvent>, Vec<String>, String)> {
        // TODO: Perform sync-collection REPORT
        // TODO: Parse response to extract added/modified and deleted events
        // TODO: Return new sync token
        todo!("Implement sync_collection")
    }
}

/// Parse calendar home URL from PROPFIND response
///
/// # Design Decision
/// We use quick-xml for parsing because:
/// - It's fast and low-level
/// - We only need to extract specific elements
/// - No need for full DOM tree
/// - Already in our dependencies
fn parse_calendar_home(xml: &str) -> Result<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_calendar_home_set = false;
    let mut in_href = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"calendar-home-set" | b"C:calendar-home-set" => in_calendar_home_set = true,
                    b"href" | b"d:href" | b"D:href" if in_calendar_home_set => in_href = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_href => {
                let text = e.unescape()?.into_owned();
                return Ok(text);
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"calendar-home-set" | b"C:calendar-home-set" => in_calendar_home_set = false,
                    b"href" | b"d:href" | b"D:href" => in_href = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Err(anyhow::anyhow!("No calendar home found in response"))
}

/// Parse calendar list from PROPFIND response
fn parse_calendars(xml: &str) -> Result<Vec<CalendarInfo>> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut calendars = Vec::new();

    let mut current_url: Option<String> = None;
    let mut current_name: Option<String> = None;
    let mut current_color: Option<String> = None;
    let mut is_calendar = false;
    let mut in_response = false;
    let mut in_href = false;
    let mut in_displayname = false;
    let mut in_color = false;
    let mut in_resourcetype = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    b"response" | b"d:response" | b"D:response" => {
                        in_response = true;
                        current_url = None;
                        current_name = None;
                        current_color = None;
                        is_calendar = false;
                    }
                    b"href" | b"d:href" | b"D:href" if in_response && current_url.is_none() => {
                        in_href = true;
                    }
                    b"displayname" | b"d:displayname" | b"D:displayname" => {
                        in_displayname = true;
                    }
                    b"calendar-color" | b"CS:calendar-color" => {
                        in_color = true;
                    }
                    b"resourcetype" | b"d:resourcetype" | b"D:resourcetype" => {
                        in_resourcetype = true;
                    }
                    b"calendar" | b"C:calendar" if in_resourcetype => {
                        is_calendar = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_href {
                    current_url = Some(e.unescape()?.into_owned());
                } else if in_displayname {
                    current_name = Some(e.unescape()?.into_owned());
                } else if in_color {
                    current_color = Some(e.unescape()?.into_owned());
                }
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"response" | b"d:response" | b"D:response" => {
                        if is_calendar && current_url.is_some() {
                            calendars.push(CalendarInfo {
                                name: current_name.take().unwrap_or_else(|| "Unnamed Calendar".to_string()),
                                url: current_url.take().unwrap(),
                                color: current_color.take(),
                            });
                        }
                        in_response = false;
                    }
                    b"href" | b"d:href" | b"D:href" => in_href = false,
                    b"displayname" | b"d:displayname" | b"D:displayname" => in_displayname = false,
                    b"calendar-color" | b"CS:calendar-color" => in_color = false,
                    b"resourcetype" | b"d:resourcetype" | b"D:resourcetype" => in_resourcetype = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(calendars)
}

/// Parse event list from calendar-query REPORT response
fn parse_events(xml: &str) -> Result<Vec<CalDavEvent>> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut events = Vec::new();

    let mut current_url: Option<String> = None;
    let mut current_etag: Option<String> = None;
    let mut current_uid: Option<String> = None;
    let mut in_response = false;
    let mut in_href = false;
    let mut in_getetag = false;
    let mut in_uid_prop = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"response" | b"d:response" | b"D:response" => {
                        in_response = true;
                        current_url = None;
                        current_etag = None;
                        current_uid = None;
                    }
                    b"href" | b"d:href" | b"D:href" if in_response && current_url.is_none() => {
                        in_href = true;
                    }
                    b"getetag" | b"d:getetag" | b"D:getetag" => {
                        in_getetag = true;
                    }
                    // Check for <C:prop name="UID"> or <prop name="UID">
                    b"prop" | b"C:prop" | b"c:prop" => {
                        // Check if this prop has name="UID" attribute
                        let has_uid_attr = e.attributes().any(|a| {
                            if let Ok(a) = a {
                                a.key.as_ref() == b"name" && a.value.as_ref() == b"UID"
                            } else {
                                false
                            }
                        });
                        if has_uid_attr {
                            in_uid_prop = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_href {
                    current_url = Some(e.unescape()?.into_owned());
                } else if in_getetag {
                    let etag = e.unescape()?.into_owned();
                    // Remove quotes if present
                    current_etag = Some(etag.trim_matches('"').to_string());
                } else if in_uid_prop {
                    current_uid = Some(e.unescape()?.into_owned());
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"response" | b"d:response" | b"D:response" => {
                        if let (Some(url), Some(etag), Some(uid)) =
                            (current_url.take(), current_etag.take(), current_uid.take()) {
                            events.push(CalDavEvent { uid, url, etag });
                        }
                        in_response = false;
                    }
                    b"href" | b"d:href" | b"D:href" => in_href = false,
                    b"getetag" | b"d:getetag" | b"D:getetag" => in_getetag = false,
                    b"prop" | b"C:prop" | b"c:prop" => in_uid_prop = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[tokio::test]
    async fn test_new_client() {
        let result = CalDavClient::new(
            "https://caldav.example.com".to_string(),
            "testuser".to_string(),
            "testpass".to_string(),
        );
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discover_calendar_home() {
        let mut server = mockito::Server::new_async().await;

        // Mock the PROPFIND request to /.well-known/caldav
        let mock = server.mock("PROPFIND", "/.well-known/caldav")
            .match_header("depth", "0")
            .with_status(207)
            .with_header("content-type", "application/xml; charset=utf-8")
            .with_body(r#"<?xml version="1.0" encoding="utf-8" ?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/.well-known/caldav</href>
    <propstat>
      <prop>
        <C:calendar-home-set>
          <href>/calendars/testuser/</href>
        </C:calendar-home-set>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let calendar_home = client.discover_calendar_home().await.unwrap();
        assert!(calendar_home.contains("/calendars/testuser"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_calendars() {
        let mut server = mockito::Server::new_async().await;

        let mock = server.mock("PROPFIND", "/calendars/testuser/")
            .match_header("depth", "1")
            .with_status(207)
            .with_header("content-type", "application/xml; charset=utf-8")
            .with_body(r#"<?xml version="1.0" encoding="utf-8" ?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">
  <response>
    <href>/calendars/testuser/default/</href>
    <propstat>
      <prop>
        <resourcetype>
          <collection/>
          <C:calendar/>
        </resourcetype>
        <displayname>My Calendar</displayname>
        <CS:calendar-color>#FF0000FF</CS:calendar-color>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
  <response>
    <href>/calendars/testuser/work/</href>
    <propstat>
      <prop>
        <resourcetype>
          <collection/>
          <C:calendar/>
        </resourcetype>
        <displayname>Work Calendar</displayname>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let calendars = client.list_calendars("/calendars/testuser/").await.unwrap();
        assert_eq!(calendars.len(), 2);
        assert_eq!(calendars[0].name, "My Calendar");
        assert!(calendars[0].url.contains("/calendars/testuser/default/"));
        assert_eq!(calendars[0].color, Some("#FF0000FF".to_string()));
        assert_eq!(calendars[1].name, "Work Calendar");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_events() {
        let mut server = mockito::Server::new_async().await;

        let mock = server.mock("REPORT", "/calendars/testuser/default/")
            .match_header("depth", "1")
            .with_status(207)
            .with_header("content-type", "application/xml; charset=utf-8")
            .with_body(r#"<?xml version="1.0" encoding="utf-8" ?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/event1.ics</href>
    <propstat>
      <prop>
        <getetag>"abc123"</getetag>
        <C:calendar-data>
          <C:comp name="VCALENDAR">
            <C:comp name="VEVENT">
              <C:prop name="UID">event-uid-1</C:prop>
            </C:comp>
          </C:comp>
        </C:calendar-data>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
  <response>
    <href>/calendars/testuser/default/event2.ics</href>
    <propstat>
      <prop>
        <getetag>"def456"</getetag>
        <C:calendar-data>
          <C:comp name="VCALENDAR">
            <C:comp name="VEVENT">
              <C:prop name="UID">event-uid-2</C:prop>
            </C:comp>
          </C:comp>
        </C:calendar-data>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let events = client.list_events("/calendars/testuser/default/").await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].uid, "event-uid-1");
        assert!(events[0].url.contains("/event1.ics"));
        assert_eq!(events[0].etag, "abc123");
        assert_eq!(events[1].uid, "event-uid-2");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_event() {
        let mut server = mockito::Server::new_async().await;

        let mock = server.mock("GET", "/calendars/testuser/default/event1.ics")
            .with_status(200)
            .with_header("content-type", "text/calendar; charset=utf-8")
            .with_body(r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:event-uid-1
DTSTART:20240101T120000Z
DTEND:20240101T130000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let ical_data = client.get_event("/calendars/testuser/default/event1.ics").await.unwrap();
        assert!(ical_data.contains("UID:event-uid-1"));
        assert!(ical_data.contains("SUMMARY:Test Event"));
        mock.assert_async().await;
    }
}
