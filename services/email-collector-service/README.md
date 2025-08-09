# Email Collector Service

This microservice handles email collection through two methods:
1. **Webhook endpoint** - Receives email data from external services
2. **IMAP fetching** - Actively fetches emails from email servers

## Features

- **Dual Email Collection**: Supports both webhook-based and IMAP-based email collection
- **Automatic Processing**: Forwards emails to the channel service for AI-powered task extraction
- **Periodic Polling**: Configurable interval for checking new emails
- **Simple Email Parsing**: Custom email parser that extracts sender, subject, and body
- **Health Monitoring**: Built-in health check endpoint

## Configuration

### Environment Variables

#### Service Configuration
- `PORT` - Service port (default: 8006)
- `RUST_LOG` - Logging level (default: info)
- `CHANNEL_SERVICE_URL` - URL of the channel service (default: http://localhost:8001)

#### Email Fetching Configuration (Optional)
- `EMAIL_SERVER` - IMAP server hostname (e.g., imap.gmail.com)
- `EMAIL_PORT` - IMAP server port (default: 993)
- `EMAIL_USERNAME` - Email account username
- `EMAIL_PASSWORD` - Email account password or app password
- `EMAIL_USE_TLS` - Use TLS encryption (default: true)
- `EMAIL_POLL_INTERVAL` - Polling interval in seconds (default: 60)

### Example Configuration

```bash
# Basic service configuration
PORT=8006
RUST_LOG=info
CHANNEL_SERVICE_URL=http://localhost:8001

# Gmail IMAP configuration
EMAIL_SERVER=imap.gmail.com
EMAIL_PORT=993
EMAIL_USERNAME=your-email@gmail.com
EMAIL_PASSWORD=your-app-password
EMAIL_USE_TLS=true
EMAIL_POLL_INTERVAL=60
```

## API Endpoints

### POST /api/v1/email
Webhook endpoint for receiving email data from external services.

**Request Body:**
```json
{
  "sender": "user@example.com",
  "subject": "Optional subject line",
  "body": "Email body content",
  "case_id": "optional-uuid-for-existing-case"
}
```

**Response:**
```json
{
  "case_id": "generated-or-existing-case-uuid",
  "message": "Success message"
}
```

### GET /health
Health check endpoint.

**Response:**
```json
{
  "service": "email-collector-service",
  "status": "healthy",
  "version": "0.1.0"
}
```

## Email Provider Setup

### Gmail
1. Enable 2-factor authentication
2. Generate an "App Password" for the email collector service
3. Use the app password instead of your regular password

### Outlook/Hotmail
- Server: `outlook.office365.com`
- Port: `993`
- Use your regular credentials or app password

### Yahoo
- Server: `imap.mail.yahoo.com`
- Port: `993`
- May require app password depending on security settings

## Running the Service

### Development
```bash
cargo run
```

### Production
```bash
cargo build --release
./target/release/email-collector-service
```

### Docker
```bash
docker build -t email-collector-service .
docker run -p 8006:8006 --env-file .env email-collector-service
```

## Email Processing Flow

1. **Email Collection**:
   - IMAP: Service polls email server every `EMAIL_POLL_INTERVAL` seconds
   - Webhook: External service posts email data to `/api/v1/email`

2. **Email Parsing**:
   - Extracts sender email address from From header
   - Extracts subject line
   - Extracts email body content

3. **Message Creation**:
   - Combines subject and body into a single message
   - Creates `MessageRequest` with `MessageChannel::Email`
   - Sets sender_id to the extracted email address

4. **Forwarding**:
   - Sends message to channel service at `/api/v1/message`
   - Channel service processes with AI for task extraction

5. **Cleanup** (IMAP only):
   - Marks processed emails as read
   - Prevents reprocessing of the same emails

## Monitoring

- Service logs all email processing activities
- Health endpoint available for monitoring systems
- Failed email processing attempts are logged with error details

## Security Considerations

- Store email passwords securely (use app passwords when possible)
- Use TLS encryption for IMAP connections
- Consider firewall rules for webhook endpoints
- Regularly rotate email credentials

## Troubleshooting

### IMAP Connection Issues
- Verify server hostname and port
- Check username/password credentials
- Ensure TLS settings match server requirements
- Check firewall/network connectivity

### Email Processing Issues
- Check channel service availability
- Verify `CHANNEL_SERVICE_URL` configuration
- Review service logs for detailed error messages

### Webhook Issues
- Verify webhook URL configuration in email provider
- Check request format matches expected schema
- Ensure service is accessible from external networks
