import React, { useState, useEffect } from 'react';
import { VoucherItem } from '../types';
import { Shield, Plus, AlertCircle, Copy, Check, Ban } from 'lucide-react';

export const AdminVouchers: React.FC = () => {
  const [vouchers, setVouchers] = useState<VoucherItem[]>([]);
  const [count, setCount] = useState<number>(1);
  const [note, setNote] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const [notice, setNotice] = useState<string>('');

  const loadVouchers = async () => {
    try {
      const res = await fetch('/admin/vouchers', { headers: { Accept: 'application/json' } });
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data.vouchers)) setVouchers(data.vouchers);
      }
    } catch {
      // Ignore
    }
  };

  useEffect(() => {
    loadVouchers();
  }, []);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setNotice('');
    try {
      const res = await fetch('/admin/vouchers', {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({ count: String(count), note }).toString(),
      });
      if (res.ok) {
        setNotice(`Berhasil mencetak ${count} voucer baru.`);
        setNote('');
        loadVouchers();
      }
    } catch {
      // Ignore
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async (code: string) => {
    if (!confirm(`Cabut voucer ${code}?`)) return;
    try {
      await fetch(`/admin/vouchers/${code}/revoke`, { method: 'POST' });
      loadVouchers();
    } catch {
      // Ignore
    }
  };

  const copyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(code);
    setTimeout(() => setCopiedCode(null), 2000);
  };

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Create Voucher Card */}
      <div className="card-bone p-6 sm:p-7">
        <h1 className="text-base font-bold text-[#0f172a] mb-4 flex items-center gap-2">
          <Shield className="w-5 h-5 text-orange-500" />
          <span>Cetak Kode Voucer Baru</span>
        </h1>

        {notice && (
          <div className="mb-4 p-3 rounded-lg bg-emerald-50 border border-emerald-200 text-emerald-800 text-xs font-semibold">
            {notice}
          </div>
        )}

        <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <div>
            <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
              Jumlah (1-100)
            </label>
            <input
              type="number"
              min="1"
              max="100"
              value={count}
              onChange={(e) => setCount(Number(e.target.value))}
              className="input-bone w-full px-3 py-2 text-sm font-bold font-mono"
            />
          </div>

          <div>
            <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
              Catatan / Peruntukan
            </label>
            <input
              type="text"
              placeholder="Misal: Giveaway TG"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              className="input-bone w-full px-3 py-2 text-sm"
            />
          </div>

          <div className="flex items-end">
            <button
              type="submit"
              disabled={loading}
              className="btn-orange w-full py-2.5 rounded-lg text-xs font-bold flex items-center justify-center gap-1.5"
            >
              <Plus className="w-4 h-4" />
              <span>{loading ? 'Mencetak...' : 'Cetak Voucer'}</span>
            </button>
          </div>
        </form>
      </div>

      {/* Voucher List Card */}
      <div className="card-bone p-6 sm:p-7">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-bold text-[#0f172a]">Daftar Voucer</h2>
          <span className="text-xs text-[#71717a]">Total: {vouchers.length}</span>
        </div>

        {vouchers.length === 0 ? (
          <p className="text-center py-8 text-xs text-[#71717a]">Belum ada voucer.</p>
        ) : (
          <div className="divide-y divide-[#e8e4dc] max-h-96 overflow-y-auto pr-1">
            {vouchers.map((v) => (
              <div key={v.code} className="py-3 flex items-center justify-between gap-3 text-xs">
                <div className="flex items-center gap-2 font-mono">
                  <span className="font-bold text-[#0f172a]">{v.code}</span>
                  <button
                    onClick={() => copyCode(v.code)}
                    className="p-1 rounded hover:bg-[#f4f1ea] text-[#71717a]"
                    title="Salin"
                  >
                    {copiedCode === v.code ? <Check className="w-3.5 h-3.5 text-emerald-600" /> : <Copy className="w-3.5 h-3.5" />}
                  </button>
                  {v.note && (
                    <span className="text-[11px] font-sans text-[#71717a] bg-[#f4f1ea] px-1.5 py-0.5 rounded">
                      {v.note}
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <span
                    className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase ${
                      v.revoked
                        ? 'bg-red-100 text-red-700'
                        : v.used_at
                        ? 'bg-gray-100 text-gray-600'
                        : 'bg-emerald-100 text-emerald-700'
                    }`}
                  >
                    {v.revoked ? 'Dicabut' : v.used_at ? 'Dipakai' : 'Aktif'}
                  </span>
                  {!v.revoked && !v.used_at && (
                    <button
                      onClick={() => handleRevoke(v.code)}
                      className="p-1 text-red-600 hover:bg-red-50 rounded"
                      title="Cabut"
                    >
                      <Ban className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
