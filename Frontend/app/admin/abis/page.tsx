"use client";

import { FormEvent, useEffect, useState } from "react";
import Navbar from "../../components/Navbar";

// Backend API URL (falls back to localhost during development)
const API_BASE_URL =
process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:3000";

// Default form values used when creating a new ABI registry entry
const EMPTY_ABI = {
  contractAddress: "",
  contractType: "",
  displayName: "",
  description: "",
  network: "stellar",
  version: "1.0.0",
  contractSchema: `{
  "type": "object",
  "properties": {
    "contractType": { "type": "string" }
  }
}`,

  abiSchema: `{
  "spec": {
    "entries": []
  }
}`,
  functionSchemas: `{
  "example_fn": {
    "inputs": [{ "name": "id", "type": "string" }],
    "outputs": [{ "name": "ok", "type": "boolean" }]
  }
}`,
  eventSchemas: `{
  "example_event": {
    "fields": {
      "id": { "type": "string" }
    }
  }
}`,
  changelog: "",
  metadata: `{
  "source": "admin-ui"
}`,
  markAsCurrent: true,
};

type RegistrySummary = {
  contractAddress: string;
  contractType: string;
  displayName: string;
  currentVersion: string;
  network: string;
  description?: string;
  version?: {
    version: string;
    abiSchema: unknown;
    contractSchema: unknown;
    functionSchemas: unknown;
    eventSchemas: unknown;
    changelog?: string;
  };
};

function prettyJson(value: unknown) {
  return JSON.stringify(value ?? {}, null, 2);
}

export default function AbiAdminPage() {
  const [token, setToken] = useState("");
  const [registries, setRegistries] = useState<RegistrySummary[]>([]);
  const [selectedAddress, setSelectedAddress] = useState("");
  const [form, setForm] = useState(EMPTY_ABI);
  const [status, setStatus] = useState("Loading ABI registry...");
  const [saving, setSaving] = useState(false);

  async function loadRegistries() {
    setStatus("Refreshing ABI registry...");
    try {
      const response = await fetch(`${API_BASE_URL}/abi-registry/contracts`, {
        cache: "no-store",
      });
      const payload = await response.json();
      const items = Array.isArray(payload) ? payload : [];
      setRegistries(items);
      setStatus(
        items.length
          ? `Loaded ${items.length} contract ABI entr${items.length === 1 ? "y" : "ies"}.`
          : "No ABIs registered yet.",
      );
    } catch (error) {
      setStatus("Unable to load ABI registry. Check the backend URL.");
    }
  }

  useEffect(() => {
    void loadRegistries();
  }, []);

  useEffect(() => {
    if (!selectedAddress) {
      return;
    }

    const selected = registries.find(
      (registry) => registry.contractAddress === selectedAddress,
    );

    if (!selected) {
      return;
    }

    setForm({
      contractAddress: selected.contractAddress,
      contractType: selected.contractType,
      displayName: selected.displayName,
      description: selected.description ?? "",
      network: selected.network ?? "stellar",
      version: selected.version?.version ?? selected.currentVersion ?? "1.0.0",
      contractSchema: prettyJson(selected.version?.contractSchema),
      abiSchema: prettyJson(selected.version?.abiSchema),
      functionSchemas: prettyJson(selected.version?.functionSchemas),
      eventSchemas: prettyJson(selected.version?.eventSchemas),
      changelog: selected.version?.changelog ?? "",
      metadata: prettyJson({ updatedFrom: "admin-ui" }),
      markAsCurrent: true,
    });
  }, [selectedAddress, registries]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setStatus("Saving ABI version...");

    try {
      const response = await fetch(`${API_BASE_URL}/abi-registry/contracts`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({
          contractAddress: form.contractAddress,
          contractType: form.contractType,
          displayName: form.displayName,
          description: form.description,
          network: form.network,
          version: form.version,
          abiSchema: JSON.parse(form.abiSchema),
          contractSchema: JSON.parse(form.contractSchema),
          functionSchemas: JSON.parse(form.functionSchemas),
          eventSchemas: JSON.parse(form.eventSchemas),
          changelog: form.changelog,
          metadata: JSON.parse(form.metadata),
          markAsCurrent: form.markAsCurrent,
        }),
      });

      if (!response.ok) {
        const failure = await response.text();
        throw new Error(failure || "Save failed");
      }

      await loadRegistries();
      setSelectedAddress(form.contractAddress);
      setStatus(`Saved ABI ${form.version} for ${form.contractAddress}.`);
    } catch (error) {
      setStatus(
        error instanceof Error
          ? `Save failed: ${error.message}`
          : "Save failed.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(59,59,240,0.22),_transparent_42%),linear-gradient(180deg,_#02030a_0%,_#090f1d_42%,_#02030a_100%)] text-white">
      <Navbar />
      <section className="mx-auto flex w-full max-w-7xl flex-col gap-8 px-4 pb-16 pt-32 sm:px-6 lg:px-8">
        <div className="grid gap-5 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="rounded-[28px] border border-white/10 bg-white/5 p-6 shadow-[0_24px_80px_rgba(0,0,0,0.28)] backdrop-blur">
            <p className="text-xs uppercase tracking-[0.4em] text-cyan-300/80">
              Soroban ABI Control Plane
            </p>
            <h1 className="mt-3 max-w-2xl text-3xl font-semibold sm:text-5xl">
              Version contract schemas without hardcoding decoders.
            </h1>
            <p className="mt-4 max-w-3xl text-sm leading-7 text-white/72 sm:text-base">
              Register contract ABIs by address, preserve older versions for
              backward compatibility, and keep invocation plus event decoding in
              sync from one registry.
            </p>
            <div className="mt-6 grid gap-3 sm:grid-cols-3">
              <div className="rounded-2xl border border-cyan-400/20 bg-cyan-400/10 p-4">
                <div className="text-2xl font-semibold">{registries.length}</div>
                <div className="mt-1 text-xs uppercase tracking-[0.28em] text-white/60">
                  Registered Contracts
                </div>
              </div>
              <div className="rounded-2xl border border-white/10 bg-black/20 p-4">
                <div className="text-2xl font-semibold">
                  {new Set(registries.map((item) => item.contractType)).size}
                </div>
                <div className="mt-1 text-xs uppercase tracking-[0.28em] text-white/60">
                  Contract Types
                </div>
              </div>
              <div className="rounded-2xl border border-white/10 bg-black/20 p-4">
                <div className="text-sm font-medium text-white/80">{status}</div>
              </div>
            </div>
          </div>

          <aside className="rounded-[28px] border border-white/10 bg-black/25 p-6">
            <label className="text-xs uppercase tracking-[0.28em] text-white/60">
              Admin Bearer Token
            </label>
            <textarea
              value={token}
              onChange={(event) => setToken(event.target.value)}
              className="mt-3 h-24 w-full rounded-2xl border border-white/10 bg-black/30 px-4 py-3 text-sm outline-none placeholder:text-white/30 focus:border-cyan-400/40"
              placeholder="Paste JWT for SUPER_ADMIN or TENANT_ADMIN"
            />
            <label className="mt-5 block text-xs uppercase tracking-[0.28em] text-white/60">
              Existing Registries
            </label>
            <div className="mt-3 space-y-3">
              {registries.map((registry) => (
                <button
                  key={registry.contractAddress}
                  type="button"
                  onClick={() => setSelectedAddress(registry.contractAddress)}
                  className={`w-full rounded-2xl border px-4 py-3 text-left transition ${
                    selectedAddress === registry.contractAddress
                      ? "border-cyan-400/50 bg-cyan-400/12"
                      : "border-white/10 bg-white/5 hover:border-white/25"
                  }`}
                >
                  <div className="text-sm font-semibold">{registry.displayName}</div>
                  <div className="mt-1 text-xs text-white/60">
                    {registry.contractAddress}
                  </div>
                  <div className="mt-2 flex items-center justify-between text-xs text-white/55">
                    <span>{registry.contractType}</span>
                    <span>v{registry.currentVersion}</span>
                  </div>
                </button>
              ))}
            </div>
          </aside>
        </div>

        <form
          onSubmit={handleSubmit}
          className="grid gap-6 rounded-[32px] border border-white/10 bg-white/5 p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)] lg:grid-cols-2"
        >
          <div className="space-y-5">
            <Field
              label="Contract Address"
              value={form.contractAddress}
              onChange={(value) => setForm((current) => ({ ...current, contractAddress: value }))}
            />
            <Field
              label="Display Name"
              value={form.displayName}
              onChange={(value) => setForm((current) => ({ ...current, displayName: value }))}
            />
            <Field
              label="Contract Type"
              value={form.contractType}
              onChange={(value) => setForm((current) => ({ ...current, contractType: value }))}
            />
            <Field
              label="Version"
              value={form.version}
              onChange={(value) => setForm((current) => ({ ...current, version: value }))}
            />
            <Field
              label="Network"
              value={form.network}
              onChange={(value) => setForm((current) => ({ ...current, network: value }))}
            />
            <Field
              label="Description"
              value={form.description}
              multiline
              onChange={(value) => setForm((current) => ({ ...current, description: value }))}
            />
            <Field
              label="Changelog"
              value={form.changelog}
              multiline
              onChange={(value) => setForm((current) => ({ ...current, changelog: value }))}
            />
          </div>

          <div className="space-y-5">
            <JsonField
              label="Contract JSON Schema"
              value={form.contractSchema}
              onChange={(value) => setForm((current) => ({ ...current, contractSchema: value }))}
            />
            <JsonField
              label="ABI Schema"
              value={form.abiSchema}
              onChange={(value) => setForm((current) => ({ ...current, abiSchema: value }))}
            />
            <JsonField
              label="Function Schemas"
              value={form.functionSchemas}
              onChange={(value) => setForm((current) => ({ ...current, functionSchemas: value }))}
            />
            <JsonField
              label="Event Schemas"
              value={form.eventSchemas}
              onChange={(value) => setForm((current) => ({ ...current, eventSchemas: value }))}
            />
            <JsonField
              label="Metadata"
              value={form.metadata}
              onChange={(value) => setForm((current) => ({ ...current, metadata: value }))}
            />
          </div>

          <div className="flex items-center gap-3 lg:col-span-2">
            <label className="inline-flex items-center gap-3 text-sm text-white/70">
              <input
                type="checkbox"
                checked={form.markAsCurrent}
                onChange={(event) =>
                  setForm((current) => ({
                    ...current,
                    markAsCurrent: event.target.checked,
                  }))
                }
              />
              Mark this version as current
            </label>
            <button
              type="submit"
              disabled={saving}
              className="rounded-full bg-cyan-300 px-6 py-3 text-sm font-semibold text-black transition hover:bg-cyan-200 disabled:cursor-not-allowed disabled:opacity-60"
            >
              {saving ? "Saving..." : "Publish ABI Version"}
            </button>
          </div>
        </form>
      </section>
    </main>
  );
}

function Field({
  label,
  value,
  onChange,
  multiline = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
}) {
  const className =
    "mt-2 w-full rounded-2xl border border-white/10 bg-black/20 px-4 py-3 text-sm outline-none placeholder:text-white/30 focus:border-cyan-400/40";

  return (
    <label className="block">
      <span className="text-xs uppercase tracking-[0.28em] text-white/60">
        {label}
      </span>
      {multiline ? (
        <textarea
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className={`${className} min-h-28`}
        />
      ) : (
        <input
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className={className}
        />
      )}
    </label>
  );
}

function JsonField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="block">
      <span className="text-xs uppercase tracking-[0.28em] text-white/60">
        {label}
      </span>
      <textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="mt-2 min-h-44 w-full rounded-2xl border border-white/10 bg-[#06101f] px-4 py-3 font-mono text-xs leading-6 text-cyan-100 outline-none focus:border-cyan-400/40"
      />
    </label>
  );
}
