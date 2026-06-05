# [Mobile] Biometric Authentication - Face ID, Touch ID, and Fingerprint Support

## 🎯 Problem Statement

The mobile app currently lacks biometric authentication, forcing users to enter credentials every time they open the app. This creates:
- Poor user experience (repeated login friction)
- Security risk (users may disable auth entirely)
- No quick access option for returning users
- Competitive disadvantage vs other mobile apps

## 💡 Solution

Implement biometric authentication using react-native-biometrics to support Face ID (iOS), Touch ID (iOS), and Fingerprint (Android). Allow users to enable/disable biometric auth in settings and use it as a quick unlock method.

## 📁 Files to Create/Modify

### New Files
```
mobile/src/services/biometrics.ts
mobile/src/hooks/useBiometrics.ts
mobile/src/components/BiometricPrompt.tsx
```

### Modified Files
```
mobile/src/screens/auth/LoginScreen.tsx
mobile/src/screens/main/SettingsScreen.tsx
mobile/src/services/auth.ts
mobile/src/store/authStore.ts
```

## 🔧 Technical Implementation

### Biometric Service
```typescript
import ReactNativeBiometrics from 'react-native-biometrics';

export class BiometricService {
  private rnBiometrics: ReactNativeBiometrics;

  constructor() {
    this.rnBiometrics = new ReactNativeBiometrics();
  }

  async isBiometricAvailable(): Promise<boolean> {
    const { available, biometryType } = await this.rnBiometrics.isSensorAvailable();
    return available && biometryType !== undefined;
  }

  async authenticate(reason: string): Promise<boolean> {
    try {
      const { success } = await this.rnBiometrics.simplePrompt({
        promptMessage: reason,
        cancelButtonText: 'Cancel',
      });
      return success;
    } catch (error) {
      console.error('Biometric authentication failed:', error);
      return false;
    }
  }

  async getBiometryType(): Promise<string | undefined> {
    const { biometryType } = await this.rnBiometrics.isSensorAvailable();
    return biometryType;
  }
}

export const biometricService = new BiometricService();
```

### Custom Hook
```typescript
export const useBiometrics = () => {
  const [isAvailable, setIsAvailable] = useState(false);
  const [biometryType, setBiometryType] = useState<string>();

  useEffect(() => {
    checkBiometricAvailability();
  }, []);

  const checkBiometricAvailability = async () => {
    const available = await biometricService.isBiometricAvailable();
    setIsAvailable(available);
    
    if (available) {
      const type = await biometricService.getBiometryType();
      setBiometryType(type);
    }
  };

  const authenticate = async (reason: string) => {
    return await biometricService.authenticate(reason);
  };

  return { isAvailable, biometryType, authenticate };
};
```

### Login Screen Integration
```typescript
export const LoginScreen = () => {
  const { isAvailable, biometryType, authenticate } = useBiometrics();
  const { login } = useAuthStore();

  const handleBiometricLogin = async () => {
    const success = await authenticate('Authenticate to access Stellar Insights');
    
    if (success) {
      // Retrieve stored credentials and login
      const credentials = await getStoredCredentials();
      if (credentials) {
        await login(credentials);
      }
    }
  };

  return (
    <View>
      {isAvailable && (
        <TouchableOpacity onPress={handleBiometricLogin}>
          <Text>Login with {biometryType}</Text>
        </TouchableOpacity>
      )}
    </View>
  );
};
```

## ✅ Acceptance Criteria

### Biometric Detection
- [ ] Detect if biometric hardware is available
- [ ] Identify biometry type (Face ID, Touch ID, Fingerprint)
- [ ] Handle devices without biometric support gracefully
- [ ] Check if biometrics are enrolled

### Authentication Flow
- [ ] Prompt user for biometric authentication
- [ ] Handle successful authentication
- [ ] Handle failed authentication (wrong biometric)
- [ ] Handle cancelled authentication (user cancels)
- [ ] Handle biometric lockout (too many failures)
- [ ] Fallback to password/PIN on biometric failure

### Settings Integration
- [ ] Toggle to enable/disable biometric auth
- [ ] Show current biometry type in settings
- [ ] Require authentication to enable biometrics
- [ ] Clear biometric preference on logout
- [ ] Show appropriate UI for unavailable biometrics

### Security
- [ ] Never store passwords in plain text
- [ ] Use secure storage for credentials
- [ ] Require re-authentication after timeout
- [ ] Handle biometric changes (new fingerprint added)
- [ ] Respect system biometric settings

### Platform Support
- [ ] Works on iOS with Face ID
- [ ] Works on iOS with Touch ID
- [ ] Works on Android with Fingerprint
- [ ] Handles platform-specific UI differences
- [ ] Respects platform biometric guidelines

### Error Handling
- [ ] Handle biometric hardware errors
- [ ] Handle enrollment errors
- [ ] Handle authentication errors
- [ ] Show user-friendly error messages
- [ ] Log errors for debugging

## 🧪 Testing Strategy

### Manual Testing
1. **iOS Face ID**
   - Test on device with Face ID
   - Test successful authentication
   - Test failed authentication
   - Test cancelled authentication

2. **iOS Touch ID**
   - Test on device with Touch ID
   - Test with registered fingerprint
   - Test with unregistered fingerprint

3. **Android Fingerprint**
   - Test on Android device
   - Test with registered fingerprint
   - Test with unregistered fingerprint

### Unit Tests
```typescript
describe('BiometricService', () => {
  it('detects biometric availability', async () => {
    const available = await biometricService.isBiometricAvailable();
    expect(typeof available).toBe('boolean');
  });

  it('authenticates successfully', async () => {
    const success = await biometricService.authenticate('Test');
    expect(typeof success).toBe('boolean');
  });
});
```

## 🔗 Dependencies

**Depends on**: Secure Token Storage
**Required by**: Login Screen, Settings Screen

## ⏱️ Estimated Effort

**Total: 6-8 hours**
- Biometric service implementation: 2 hours
- Login screen integration: 1 hour
- Settings integration: 1 hour
- Error handling: 1 hour
- Testing (iOS): 1 hour
- Testing (Android): 1 hour
- Documentation: 1 hour

## ✔️ Definition of Done

- [ ] Biometric authentication works on iOS (Face ID/Touch ID)
- [ ] Biometric authentication works on Android (Fingerprint)
- [ ] Settings toggle implemented
- [ ] Error handling comprehensive
- [ ] Fallback to password works
- [ ] Security best practices followed
- [ ] Tested on real devices
- [ ] Documentation complete
- [ ] Code reviewed and approved
