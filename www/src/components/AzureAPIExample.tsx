import React, { useState } from 'react';
import { useAzureAuth } from '../hooks/useAzureAuth';

/**
 * Example component showing how to use Azure authentication to call Azure APIs
 */
export const AzureAPIExample: React.FC = () => {
  const [subscriptions, setSubscriptions] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { getAzureManagementToken, getMicrosoftGraphToken, isAuthenticated } = useAzureAuth();

  const listSubscriptions = async () => {
    if (!isAuthenticated) {
      setError('Please sign in first');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // Get access token for Azure Management API
      const token = await getAzureManagementToken();
      
      if (!token) {
        throw new Error('Failed to acquire access token');
      }

      // Call Azure Management API to list subscriptions
      const response = await fetch('https://management.azure.com/subscriptions?api-version=2020-01-01', {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`API call failed: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      setSubscriptions(data.value || []);
    } catch (err) {
      console.error('Error listing subscriptions:', err);
      setError(err instanceof Error ? err.message : 'Failed to list subscriptions');
    } finally {
      setLoading(false);
    }
  };

  const callGraphAPI = async () => {
    if (!isAuthenticated) {
      setError('Please sign in first');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // Get access token for Microsoft Graph API
      const token = await getMicrosoftGraphToken();
      
      if (!token) {
        throw new Error('Failed to acquire Graph API access token');
      }

      // Call Microsoft Graph API to get user profile
      const response = await fetch('https://graph.microsoft.com/v1.0/me', {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`Graph API call failed: ${response.status} ${response.statusText}`);
      }

      const userData = await response.json();
      console.log('User data from Graph API:', userData);
      setError(null);
    } catch (err) {
      console.error('Error calling Graph API:', err);
      setError(err instanceof Error ? err.message : 'Failed to call Graph API');
    } finally {
      setLoading(false);
    }
  };

  if (!isAuthenticated) {
    return (
      <div className="api-example">
        <h3>Azure API Examples</h3>
        <p>Please sign in to try Azure API calls.</p>
      </div>
    );
  }

  return (
    <div className="api-example">
      <h3>Azure API Examples</h3>
      
      <div className="api-actions">
        <button 
          onClick={listSubscriptions} 
          disabled={loading}
          className="api-button"
        >
          {loading ? 'Loading...' : 'List Azure Subscriptions'}
        </button>
        
        <button 
          onClick={callGraphAPI} 
          disabled={loading}
          className="api-button"
        >
          {loading ? 'Loading...' : 'Get User Profile (Graph API)'}
        </button>
      </div>

      {error && (
        <div className="api-error">
          <strong>Error:</strong> {error}
        </div>
      )}

      {subscriptions.length > 0 && (
        <div className="subscriptions-list">
          <h4>Azure Subscriptions:</h4>
          <ul>
            {subscriptions.map((sub) => (
              <li key={sub.subscriptionId}>
                <strong>{sub.displayName}</strong> ({sub.subscriptionId})
                <br />
                <small>State: {sub.state}</small>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};