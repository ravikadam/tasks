# Office 365 OAuth2 Setup Guide

## Step 1: Create Azure App Registration

1. Go to [Azure Portal](https://portal.azure.com)
2. Navigate to **Azure Active Directory** → **App registrations** → **New registration**
3. Fill in the details:
   - **Name**: `Task Manager Email Integration`
   - **Supported account types**: `Accounts in this organizational directory only`
   - **Redirect URI**: `Web` → `http://localhost:8006/oauth/callback`

## Step 2: Configure API Permissions

1. In your app registration, go to **API permissions**
2. Click **Add a permission** → **Microsoft Graph** → **Delegated permissions**
3. Add these permissions:
   - `IMAP.AccessAsUser.All` (for IMAP access)
   - `Mail.Read` (for reading emails)
   - `offline_access` (for refresh tokens)
4. Click **Grant admin consent** (if you have admin rights)

## Step 3: Create Client Secret

1. Go to **Certificates & secrets** → **Client secrets** → **New client secret**
2. Add description: `Task Manager Email Access`
3. Set expiration: `24 months`
4. **Copy the secret value immediately** (you won't see it again)

## Step 4: Get Required Values

From your Azure app registration, copy these values:

- **Application (client) ID**: Found on the Overview page
- **Directory (tenant) ID**: Found on the Overview page  
- **Client secret**: The value you copied in Step 3

## Step 5: Configure Environment Variables

Create a `.env` file in the dashboard service directory with:

```bash
# Azure OAuth2 Configuration
AZURE_CLIENT_ID=your-application-client-id
AZURE_CLIENT_SECRET=your-client-secret-value
AZURE_TENANT_ID=your-directory-tenant-id

# Email Service URL (for token passing)
EMAIL_SERVICE_URL=http://localhost:8007
```

## Step 6: Test the OAuth Flow

1. Start the dashboard service: `docker compose up dashboard-service`
2. Visit: http://localhost:8006
3. Click **"📧 Connect Email"** button
4. Complete the OAuth flow with your Office 365 credentials
5. Check email service logs for successful token reception

## Security Notes

- **Client Secret**: Keep this secure and never commit to version control
- **Redirect URI**: Must exactly match what's configured in Azure
- **Permissions**: Only request the minimum permissions needed
- **Token Storage**: In production, store tokens securely with encryption

## Troubleshooting

- **"AADSTS50011"**: Redirect URI mismatch - check Azure configuration
- **"AADSTS65001"**: User consent required - ensure permissions are granted
- **"Invalid client"**: Check client ID and secret values
- **IMAP errors**: Ensure IMAP.AccessAsUser.All permission is granted and consented
