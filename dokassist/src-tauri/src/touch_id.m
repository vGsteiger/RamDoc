// touch_id.m — synchronous Touch ID / device-passcode prompt via LocalAuthentication.
// Called from Rust FFI.  Compiled as Objective-C ARC by build.rs.

#import <LocalAuthentication/LocalAuthentication.h>
#include <dispatch/dispatch.h>

// Return codes
#define TOUCH_ID_OK        0
#define TOUCH_ID_CANCELLED 1   // errLAUserCancel
#define TOUCH_ID_FALLBACK  2   // errLAUserFallback (not produced by DeviceOwnerAuthentication)
#define TOUCH_ID_UNAVAIL   3   // hardware / policy unavailable
#define TOUCH_ID_FAILED    4   // authentication failed

int authenticate_touch_id(const char *reason_cstr) {
    LAContext *ctx = [[LAContext alloc] init];
    NSError *canEvalError = nil;
    NSString *reason = [NSString stringWithUTF8String:reason_cstr];

    // DeviceOwnerAuthentication offers Touch ID when it is enrolled and usable
    // and falls back to the login password otherwise: Touch ID locked out after
    // failed attempts, a closed lid with an external keyboard, no enrolled
    // fingers. The biometrics-only policy has no working password fallback —
    // its "Enter Password" button returns errLAUserFallback, so a user whose
    // fingerprint was not recognised could not use their password until Touch
    // ID had locked itself out.
    LAPolicy policy = LAPolicyDeviceOwnerAuthentication;
    if (![ctx canEvaluatePolicy:policy error:&canEvalError]) {
        return TOUCH_ID_UNAVAIL;
    }

    // evaluatePolicy:localizedReason:reply: is async — bridge to synchronous
    // using a dispatch semaphore so the calling Rust thread blocks.
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block int result = TOUCH_ID_FAILED;

    [ctx evaluatePolicy:policy
       localizedReason:reason
                 reply:^(BOOL granted, NSError *error) {
        if (granted) {
            result = TOUCH_ID_OK;
        } else if (error) {
            switch (error.code) {
                case LAErrorUserCancel:
                case LAErrorSystemCancel:
                case LAErrorAppCancel:
                    result = TOUCH_ID_CANCELLED;
                    break;
                case LAErrorPasscodeNotSet:
                    result = TOUCH_ID_UNAVAIL;
                    break;
                default:
                    result = TOUCH_ID_FAILED;
                    break;
            }
        }
        dispatch_semaphore_signal(sem);
    }];

    dispatch_semaphore_wait(sem, DISPATCH_TIME_FOREVER);
    return result;
}
