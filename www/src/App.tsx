import { useState } from 'react'
import './App.css'
import { AuthProvider } from './context/AuthContext'
import { AuthenticatedContent, AuthButton, UserProfile, TokenDisplay } from './components/AuthComponents'
import { AzureCLIInterface } from './components/AzureCLIInterface'

function AppContent() {
  const [showTokens, setShowTokens] = useState(false);

  return (
    <>
      <header className="app-header">
        <h1>Azure CLI on Browser</h1>
        <AuthButton />
      </header>

      <main className="app-main">
        <AuthenticatedContent
          fallback={
            <div className="welcome-message">
              <h2>Welcome to Azure CLI Browser</h2>
              <p>Sign in with your Microsoft account to access Azure resources and manage your cloud infrastructure.</p>
            </div>
          }
        >
          <div className="authenticated-content">
            <UserProfile />
            
            <AzureCLIInterface />
            
            <div className="features-section">
              <h2>Available Features</h2>
              <p>You are now authenticated and can access Azure resources!</p>
              
              <div className="token-section">
                <button 
                  onClick={() => setShowTokens(!showTokens)}
                  className="toggle-tokens-btn"
                >
                  {showTokens ? 'Hide' : 'Show'} Access Tokens
                </button>
                {showTokens && <TokenDisplay />}
              </div>
              
              <div className="next-steps">
                <h3>Next Steps</h3>
                <ul>
                  <li>Use the access tokens to call Azure REST APIs</li>
                  <li>Manage Azure resources programmatically</li>
                  <li>Access Microsoft Graph API for user data</li>
                  <li>Integrate with Azure services like Key Vault, Storage, etc.</li>
                </ul>
              </div>
            </div>
          </div>
        </AuthenticatedContent>
      </main>
    </>
  )
}

function App() {
  return (
    <AuthProvider>
      <AppContent />
    </AuthProvider>
  )
}

export default App
