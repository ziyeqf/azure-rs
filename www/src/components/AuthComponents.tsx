import React from 'react';
import { useAuthStatus, useAzureAuth } from '../hooks/useAzureAuth';

interface AuthenticatedContentProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

/**
 * Component that only renders children if user is authenticated
 * Shows login UI if not authenticated
 */
export const AuthenticatedContent: React.FC<AuthenticatedContentProps> = ({ 
  children, 
  fallback 
}) => {
  const { isAuthenticated, loading } = useAuthStatus();

  if (loading) {
    return (
      <div className="auth-loading">
        <div>Loading authentication...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return fallback ? <>{fallback}</> : <LoginPrompt />;
  }

  return <>{children}</>;
};

/**
 * Login/Logout button component
 */
export const AuthButton: React.FC = () => {
  const { isAuthenticated, loading, error } = useAuthStatus();
  const { login, logout } = useAzureAuth();

  const handleAuthAction = async () => {
    if (isAuthenticated) {
      await logout();
    } else {
      await login();
    }
  };

  return (
    <div className="auth-button-container">
      <button 
        onClick={handleAuthAction}
        disabled={loading}
        className={`auth-button ${isAuthenticated ? 'logout' : 'login'}`}
      >
        {loading ? 'Loading...' : isAuthenticated ? 'Sign Out' : 'Sign In with Microsoft'}
      </button>
      {error && (
        <div className="auth-error">
          Error: {error}
        </div>
      )}
    </div>
  );
};

/**
 * User profile display component
 */
export const UserProfile: React.FC = () => {
  const { account, userName, userEmail } = useAuthStatus();

  if (!account) {
    return null;
  }

  return (
    <div className="user-profile">
      <div className="user-info">
        <h3>Welcome, {userName}!</h3>
        <p>Email: {userEmail}</p>
        <p>Account ID: {account.homeAccountId}</p>
      </div>
    </div>
  );
};

/**
 * Login prompt component
 */
export const LoginPrompt: React.FC = () => {
  return (
    <div className="login-prompt">
      <h2>Authentication Required</h2>
      <p>Please sign in to access Azure resources.</p>
      <AuthButton />
    </div>
  );
};

/**
 * Token display component for debugging
 */
export const TokenDisplay: React.FC = () => {
  const [tokens, setTokens] = React.useState<Record<string, string | null>>({});
  const [loading, setLoading] = React.useState(false);
  const { 
    getAzureManagementToken, 
    getMicrosoftGraphToken, 
    getKeyVaultToken, 
    getStorageToken 
  } = useAzureAuth();

  const fetchTokens = async () => {
    setLoading(true);
    try {
      const [mgmtToken, graphToken, kvToken, storageToken] = await Promise.all([
        getAzureManagementToken(),
        getMicrosoftGraphToken(),
        getKeyVaultToken(),
        getStorageToken(),
      ]);

      setTokens({
        management: mgmtToken,
        graph: graphToken,
        keyVault: kvToken,
        storage: storageToken,
      });
    } catch (error) {
      console.error('Error fetching tokens:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="token-display">
      <h3>Access Tokens (for debugging)</h3>
      <button onClick={fetchTokens} disabled={loading}>
        {loading ? 'Fetching...' : 'Fetch Tokens'}
      </button>
      {Object.entries(tokens).map(([service, token]) => (
        <div key={service} className="token-item">
          <strong>{service}:</strong>
          <code>{token ? `${token.substring(0, 50)}...` : 'Not available'}</code>
        </div>
      ))}
    </div>
  );
};