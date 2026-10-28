// Interface emitted by the pinned wasm-bindgen tool. There is no JavaScript crypto fallback.
export default function init(): Promise<unknown>;
export class BrowserPeer {
  constructor(invitation: string, origin: string, now: bigint);
  metadata(): string;
  begin(now: bigint): Uint8Array;
  acceptHandshake(bytes: Uint8Array, now: bigint): Uint8Array;
  encrypt(bytes: Uint8Array, now: bigint): Uint8Array;
  decrypt(bytes: Uint8Array, now: bigint): Uint8Array;
  free(): void;
}
