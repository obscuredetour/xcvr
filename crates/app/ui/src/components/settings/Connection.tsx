import Server from 'lucide-solid/icons/server';

import { Callout, Heading } from './parts';

/** Host, port and credentials, and Test Connection. Built with the RPC client (phases 2–3). */
export function ConnectionSettings() {
  return (
    <>
      <Heading title="Connection">The Transmission host this app talks to.</Heading>
      <Callout icon={Server} title="Not set up yet">
        <p>
          Host, port, username and password, with a Test Connection button, arrive with the RPC client. The password
          will be kept in the system keychain.
        </p>
      </Callout>
    </>
  );
}
