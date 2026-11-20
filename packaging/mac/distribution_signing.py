"""Developer ID signing for Mac distribution, using credentials in the Mac keychain."""
import json
import os
import re
import subprocess
from pathlib import Path


def signing_config():
    identity = os.environ.get('ML_MAC_SIGNING_IDENTITY', '').strip()
    profile = os.environ.get('ML_MAC_NOTARY_PROFILE', '').strip()
    if not identity and not profile:
        return None
    if not identity or not profile:
        raise ValueError('Set both the Developer ID identity and notary keychain profile')
    identities = subprocess.check_output(['security', 'find-identity', '-v', '-p', 'codesigning'], text=True)
    matches = [fingerprint for fingerprint, name in
               re.findall(r'\b([A-Fa-f0-9]{40}) "([^"]+)"', identities)
               if name.startswith('Developer ID Application: ') and
               (fingerprint.lower() == identity.lower() or name == identity)]
    if len(matches) != 1:
        raise ValueError('Configured Developer ID Application identity is missing, expired, or ambiguous')
    return matches[0], profile


def sign_app(app, fingerprint):
    entitlements = Path(__file__).with_name('host-entitlements.plist')
    subprocess.run(['codesign', '--force', '--sign', fingerprint, '--timestamp', '--options', 'runtime',
                    '--entitlements', entitlements, app], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', app], check=True)


def notarize_dmg(dmg, fingerprint, profile):
    subprocess.run(['codesign', '--force', '--sign', fingerprint, '--timestamp', dmg], check=True)
    subprocess.run(['codesign', '--verify', '--strict', dmg], check=True)
    reply = subprocess.check_output(['xcrun', 'notarytool', 'submit', dmg, '--keychain-profile', profile,
                                     '--wait', '--output-format', 'json'], text=True)
    submission = json.loads(reply)
    if submission.get('status') != 'Accepted':
        raise RuntimeError(f'Apple notarization returned {submission.get("status")} for {submission.get("id")}')
    subprocess.run(['xcrun', 'stapler', 'staple', dmg], check=True)
    subprocess.run(['xcrun', 'stapler', 'validate', dmg], check=True)
    return submission['id']

if __name__ == '__main__':
    signing = signing_config()
    if signing:
        subprocess.run(['xcrun', 'notarytool', 'history', '--keychain-profile', signing[1],
                        '--output-format', 'json'], stdout=subprocess.DEVNULL, check=True)
