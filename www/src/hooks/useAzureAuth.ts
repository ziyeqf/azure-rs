import { useAuth } from '../context/AuthContext';
import { azureScopes } from '../config/authConfig';

/**
 * Custom hook for Azure authentication operations
 */
export const useAzureAuth = () => {
  const auth = useAuth();

  /**
   * Get access token for Azure Resource Manager API
   * This token can be used to manage Azure resources
   */
  const getAzureManagementToken = async (): Promise<string | null> => {
    return auth.getAccessToken(azureScopes.management);
  };

  /**
   * Get access token for Microsoft Graph API
   * This token can be used to access user profile and other Graph APIs
   */
  const getMicrosoftGraphToken = async (): Promise<string | null> => {
    return auth.getAccessToken(azureScopes.graph);
  };

  /**
   * Get access token for Azure Key Vault
   * This token can be used to access Key Vault secrets, keys, and certificates
   */
  const getKeyVaultToken = async (): Promise<string | null> => {
    return auth.getAccessToken(azureScopes.keyVault);
  };

  /**
   * Get access token for Azure Storage
   * This token can be used to access Azure Storage services
   */
  const getStorageToken = async (): Promise<string | null> => {
    return auth.getAccessToken(azureScopes.storage);
  };

  /**
   * Get access token for custom scopes
   * @param scopes - Array of scope strings
   */
  const getCustomToken = async (scopes: string[]): Promise<string | null> => {
    return auth.getAccessToken(scopes);
  };

  return {
    ...auth,
    getAzureManagementToken,
    getMicrosoftGraphToken,
    getKeyVaultToken,
    getStorageToken,
    getCustomToken,
  };
};

/**
 * Custom hook for checking authentication status
 */
export const useAuthStatus = () => {
  const { isAuthenticated, account, loading, error } = useAuth();

  return {
    isAuthenticated,
    account,
    loading,
    error,
    isReady: !loading,
    hasError: !!error,
    userName: account?.name || account?.username,
    userEmail: account?.username,
  };
};