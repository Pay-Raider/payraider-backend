# Corridor Analytics API Examples

## Base URL
```
http://localhost:8080
```

## 1. List All Corridors (Default: Sorted by Success Rate)

### Request
```bash
curl -X GET "http://localhost:8080/api/corridors"
```

### Response
```json
{
  "corridors": [
    {
      "asset_pair": "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2 -> USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "asset_a_code": "EURC",
      "asset_a_issuer": "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2",
      "asset_b_code": "USDC",
      "asset_b_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "success_rate": 98.0,
      "total_transactions": 2500,
      "successful_transactions": 2450,
      "failed_transactions": 50,
      "volume_usd": 2500000.0,
      "last_updated": "2024-01-22 15:30:00 UTC"
    },
    {
      "asset_pair": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN -> USDT:GCQTGZQQ5G4PTM2GL7CDIFKUBIPEC52BROAQIAPW53XBRJVN6ZJVTG6V",
      "asset_a_code": "USDC",
      "asset_a_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "asset_b_code": "USDT",
      "asset_b_issuer": "GCQTGZQQ5G4PTM2GL7CDIFKUBIPEC52BROAQIAPW53XBRJVN6ZJVTG6V",
      "success_rate": 97.0,
      "total_transactions": 8000,
      "successful_transactions": 7760,
      "failed_transactions": 240,
      "volume_usd": 15000000.0,
      "last_updated": "2024-01-22 15:30:00 UTC"
    }
  ],
  "total": 10
}
```

## 2. List Corridors Sorted by Volume

### Request
```bash
curl -X GET "http://localhost:8080/api/corridors?sort_by=volume"
```

### Response
```json
{
  "corridors": [
    {
      "asset_pair": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN -> USDT:GCQTGZQQ5G4PTM2GL7CDIFKUBIPEC52BROAQIAPW53XBRJVN6ZJVTG6V",
      "success_rate": 97.0,
      "volume_usd": 15000000.0,
      "total_transactions": 8000
    },
    {
      "asset_pair": "USD:GCKFBEIYTKP5RDBQMU2GSY6PEHPBQHQT3TGZUBCEABN5DGFMXQSCMWBK -> XLM:native",
      "success_rate": 85.0,
      "volume_usd": 10000000.0,
      "total_transactions": 5000
    }
  ],
  "total": 10
}
```

## 3. List Corridors with Pagination

### Request
```bash
curl -X GET "http://localhost:8080/api/corridors?limit=5&offset=0"
```

### Response
```json
{
  "corridors": [
    // First 5 corridors
  ],
  "total": 5
}
```

## 4. Get Specific Corridor by Asset Pair

### Request
```bash
curl -X GET "http://localhost:8080/api/corridors/USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"
```

### Response
```json
{
  "asset_pair": "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2 -> USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "asset_a_code": "EURC",
  "asset_a_issuer": "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2",
  "asset_b_code": "USDC",
  "asset_b_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "success_rate": 98.0,
  "total_transactions": 2500,
  "successful_transactions": 2450,
  "failed_transactions": 50,
  "volume_usd": 2500000.0,
  "last_updated": "2024-01-22 15:30:00 UTC"
}
```

## 5. Get Corridor with Spaces in Asset Pair

### Request (URL encoded)
```bash
curl -X GET "http://localhost:8080/api/corridors/USDC%3AGA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN%20-%3E%20EURC%3AGDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"
```

### Raw URL (before encoding)
```
USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN -> EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2
```

## Error Responses

### 404 Not Found
```bash
curl -X GET "http://localhost:8080/api/corridors/NONEXISTENT:issuer->FAKE:issuer"
```

```json
{
  "error": "Corridor with asset pair NONEXISTENT:issuer->FAKE:issuer not found"
}
```

### 400 Bad Request
```bash
curl -X GET "http://localhost:8080/api/corridors/INVALID-FORMAT"
```

```json
{
  "error": "Invalid asset pair format. Expected: 'ASSET_A:ISSUER_A->ASSET_B:ISSUER_B'"
}
```

## Frontend Integration Examples

### JavaScript/TypeScript
```typescript
// Fetch all corridors sorted by success rate
async function fetchCorridors(sortBy = 'success_rate', limit = 50, offset = 0) {
  const response = await fetch(
    `http://localhost:8080/api/corridors?sort_by=${sortBy}&limit=${limit}&offset=${offset}`
  );
  
  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  
  return await response.json();
}

// Fetch specific corridor
async function fetchCorridor(assetPair: string) {
  const encodedPair = encodeURIComponent(assetPair);
  const response = await fetch(`http://localhost:8080/api/corridors/${encodedPair}`);
  
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Corridor not found');
    }
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  
  return await response.json();
}

// Usage examples
fetchCorridors('volume', 10, 0)
  .then(data => console.log('Top 10 corridors by volume:', data))
  .catch(error => console.error('Error:', error));

fetchCorridor('USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2')
  .then(corridor => console.log('USDC-EURC corridor:', corridor))
  .catch(error => console.error('Error:', error));
```

### React Hook Example
```typescript
import { useState, useEffect } from 'react';

interface Corridor {
  asset_pair: string;
  asset_a_code: string;
  asset_a_issuer: string;
  asset_b_code: string;
  asset_b_issuer: string;
  success_rate: number;
  total_transactions: number;
  successful_transactions: number;
  failed_transactions: number;
  volume_usd: number;
  last_updated: string;
}

interface CorridorsResponse {
  corridors: Corridor[];
  total: number;
}

function useCorridors(sortBy: 'success_rate' | 'volume' = 'success_rate') {
  const [data, setData] = useState<CorridorsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchData() {
      try {
        setLoading(true);
        const response = await fetch(
          `http://localhost:8080/api/corridors?sort_by=${sortBy}`
        );
        
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        
        const result = await response.json();
        setData(result);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    }

    fetchData();
  }, [sortBy]);

  return { data, loading, error };
}

// Usage in component
function CorridorsList() {
  const { data, loading, error } = useCorridors('success_rate');

  if (loading) return <div>Loading corridors...</div>;
  if (error) return <div>Error: {error}</div>;
  if (!data) return <div>No data available</div>;

  return (
    <div>
      <h2>Payment Corridors ({data.total} total)</h2>
      {data.corridors.map((corridor, index) => (
        <div key={index} className="corridor-card">
          <h3>{corridor.asset_pair}</h3>
          <p>Success Rate: {corridor.success_rate}%</p>
          <p>Volume: ${corridor.volume_usd.toLocaleString()}</p>
          <p>Transactions: {corridor.total_transactions}</p>
        </div>
      ))}
    </div>
  );
}
```

## Query Parameters Summary

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Maximum number of results to return |
| `offset` | integer | 0 | Number of results to skip (for pagination) |
| `sort_by` | string | "success_rate" | Sort by "success_rate" or "volume" |

## Asset Pair Format

The asset pair parameter must follow this format:
```
ASSET_CODE:ISSUER_ADDRESS->ASSET_CODE:ISSUER_ADDRESS
```

Examples:
- `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2`
- `XLM:native->USD:GCKFBEIYTKP5RDBQMU2GSY6PEHPBQHQT3TGZUBCEABN5DGFMXQSCMWBK`

**Note**: The API automatically normalizes asset pairs by sorting them alphabetically, so `USDC:issuer->EURC:issuer` and `EURC:issuer->USDC:issuer` refer to the same corridor.