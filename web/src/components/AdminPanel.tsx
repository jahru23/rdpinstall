import React, { useState, useEffect } from 'react';
import {
  Shield,
  Server,
  Users,
  CreditCard,
  Ticket,
  Plus,
  Copy,
  Check,
  Ban,
  Trash2,
  AlertCircle,
  CheckCircle2,
  Clock,
  Search,
  Filter,
  DollarSign,
  X,
  RefreshCw,
  Terminal,
  Activity,
  Eye,
  EyeOff,
  LogOut,
} from 'lucide-react';
import { AdminStats, AdminUser, AdminPayment, JobData, VoucherItem } from '../types';

export const AdminPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'jobs' | 'users' | 'payments' | 'vouchers'>('overview');
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string>('');
  const [notice, setNotice] = useState<string>('');

  // Admin Auth state
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [adminUsername, setAdminUsername] = useState<string>('admin');
  const [adminPassword, setAdminPassword] = useState<string>('');
  const [showAdminPassword, setShowAdminPassword] = useState<boolean>(false);
  const [loggingIn, setLoggingIn] = useState<boolean>(false);
  const [loginError, setLoginError] = useState<string>('');

  // Data states
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [jobs, setJobs] = useState<JobData[]>([]);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [payments, setPayments] = useState<AdminPayment[]>([]);
  const [vouchers, setVouchers] = useState<VoucherItem[]>([]);

  // Search & Filters
  const [jobFilter, setJobFilter] = useState<string>('all');
  const [jobSearch, setJobSearch] = useState<string>('');
  const [userSearch, setUserSearch] = useState<string>('');
  const [paymentFilter, setPaymentFilter] = useState<string>('all');
  const [voucherFilter, setVoucherFilter] = useState<string>('all');
  const [voucherSearch, setVoucherSearch] = useState<string>('');

  // Create voucher form
  const [voucherCount, setVoucherCount] = useState<number>(5);
  const [voucherNote, setVoucherNote] = useState<string>('');
  const [creatingVouchers, setCreatingVouchers] = useState<boolean>(false);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);

  // Edit balance modal
  const [selectedUserForBalance, setSelectedUserForBalance] = useState<AdminUser | null>(null);
  const [balanceAmount, setBalanceAmount] = useState<number>(10000);
  const [balanceMode, setBalanceMode] = useState<'add' | 'set'>('add');
  const [balanceNote, setBalanceNote] = useState<string>('');
  const [savingBalance, setSavingBalance] = useState<boolean>(false);

  // Fetch all admin data
  const loadData = async () => {
    setLoading(true);
    setError('');
    try {
      // 1. Overview
      const rOverview = await fetch('/admin/overview', { headers: { Accept: 'application/json' } });
      if (rOverview.status === 401 || rOverview.status === 403) {
        setIsAuthenticated(false);
        setLoading(false);
        return;
      }
      if (rOverview.ok) {
        const d = await rOverview.json();
        setStats(d.stats);
        if (Array.isArray(d.vouchers)) setVouchers(d.vouchers);
        setIsAuthenticated(true);
      } else {
        setIsAuthenticated(false);
        setLoading(false);
        return;
      }

      // 2. Jobs
      const rJobs = await fetch('/admin/jobs', { headers: { Accept: 'application/json' } });
      if (rJobs.ok) {
        const d = await rJobs.json();
        if (Array.isArray(d.jobs)) setJobs(d.jobs);
      }

      // 3. Users
      const rUsers = await fetch('/admin/users', { headers: { Accept: 'application/json' } });
      if (rUsers.ok) {
        const d = await rUsers.json();
        if (Array.isArray(d.users)) setUsers(d.users);
      }

      // 4. Payments
      const rPayments = await fetch('/admin/payments', { headers: { Accept: 'application/json' } });
      if (rPayments.ok) {
        const d = await rPayments.json();
        if (Array.isArray(d.payments)) setPayments(d.payments);
      }
    } catch {
      setError('Gagal memuat data panel admin.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const showNotice = (msg: string) => {
    setNotice(msg);
    setTimeout(() => setNotice(''), 4000);
  };

  // Actions: Jobs
  const handleCancelJob = async (id: number) => {
    if (!window.confirm(`Batalkan paksa instalasi #${id}?`)) return;
    try {
      const res = await fetch(`/admin/jobs/${id}/cancel`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice(data.message || 'Job berhasil dibatalkan.');
        loadData();
      } else {
        setError(data.message || 'Gagal membatalkan job.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  const handleDeleteJob = async (id: number) => {
    if (!window.confirm(`Hapus permanen riwayat job #${id}?`)) return;
    try {
      const res = await fetch(`/admin/jobs/${id}/delete`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice('Job berhasil dihapus.');
        loadData();
      } else {
        setError(data.message || 'Gagal menghapus job.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  // Actions: Users
  const handleSaveBalance = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedUserForBalance) return;
    setSavingBalance(true);
    try {
      const form = new URLSearchParams();
      form.append('amount', String(balanceAmount));
      form.append('mode', balanceMode);
      form.append('note', balanceNote);

      const res = await fetch(`/admin/users/${selectedUserForBalance.id}/balance`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: form.toString(),
      });
      const data = await res.json();
      if (data.success) {
        showNotice(`Saldo user #${selectedUserForBalance.id} berhasil diperbarui.`);
        setSelectedUserForBalance(null);
        setBalanceNote('');
        loadData();
      } else {
        setError(data.error || 'Gagal memperbarui saldo.');
      }
    } catch {
      setError('Gagal memperbarui saldo.');
    } finally {
      setSavingBalance(false);
    }
  };

  const handleToggleBan = async (id: number, currentBanned: number) => {
    const action = currentBanned ? 'buka blokir' : 'blokir';
    if (!window.confirm(`Yakin ingin ${action} user #${id}?`)) return;
    try {
      const res = await fetch(`/admin/users/${id}/toggle-ban`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice(`Status blokir user #${id} berhasil diubah.`);
        loadData();
      } else {
        setError('Gagal mengubah status blokir.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  // Actions: Payments
  const handleApprovePayment = async (reference: string, amount: number) => {
    if (!window.confirm(`Setujui manual pembayaran PG-${reference} sebesar Rp${amount.toLocaleString('id-ID')}? Saldo user akan otomatis bertambah.`)) {
      return;
    }
    try {
      const res = await fetch(`/admin/payments/${reference}/approve`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice(data.message || 'Pembayaran berhasil disetujui.');
        loadData();
      } else {
        setError(data.message || 'Gagal menyetujui pembayaran.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  // Actions: Vouchers
  const handleCreateVouchers = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreatingVouchers(true);
    try {
      const form = new URLSearchParams();
      form.append('count', String(voucherCount));
      form.append('note', voucherNote);

      const res = await fetch('/admin/vouchers', {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: form.toString(),
      });
      const data = await res.json();
      if (data.success) {
        showNotice(`Berhasil mencetak ${data.count} voucer baru.`);
        setVoucherNote('');
        loadData();
      } else {
        setError('Gagal mencetak voucer.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    } finally {
      setCreatingVouchers(false);
    }
  };

  const handleRevokeVoucher = async (code: string) => {
    if (!window.confirm(`Cabut voucer ${code}? Voucer ini tidak akan bisa digunakan lagi.`)) return;
    try {
      const res = await fetch(`/admin/vouchers/${code}/revoke`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice(`Voucer ${code} berhasil dicabut.`);
        loadData();
      } else {
        setError(data.message || 'Gagal mencabut voucer.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  const handleDeleteVoucher = async (code: string) => {
    if (!window.confirm(`Hapus voucer ${code} dari database?`)) return;
    try {
      const res = await fetch(`/admin/vouchers/${code}/delete`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        showNotice(`Voucer ${code} berhasil dihapus.`);
        loadData();
      } else {
        setError(data.message || 'Gagal menghapus voucer.');
      }
    } catch {
      setError('Terjadi kesalahan jaringan.');
    }
  };

  const copyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(code);
    setTimeout(() => setCopiedCode(null), 2000);
  };

  // Filtered lists
  const filteredJobs = jobs.filter((j) => {
    if (jobFilter === 'running' && j.status !== 'running' && j.status !== 'queued') return false;
    if (jobFilter === 'done' && j.status !== 'done') return false;
    if (jobFilter === 'failed' && j.status !== 'failed' && j.status !== 'cancelled') return false;
    if (jobSearch.trim()) {
      const q = jobSearch.toLowerCase();
      return (
        String(j.id).includes(q) ||
        (j.host || '').toLowerCase().includes(q) ||
        (j.os_label || '').toLowerCase().includes(q)
      );
    }
    return true;
  });

  const filteredUsers = users.filter((u) => {
    if (!userSearch.trim()) return true;
    const q = userSearch.toLowerCase();
    return (
      String(u.id).includes(q) ||
      (u.username || '').toLowerCase().includes(q) ||
      (u.first_name || '').toLowerCase().includes(q)
    );
  });

  const filteredPayments = payments.filter((p) => {
    if (paymentFilter === 'paid' && p.status !== 'paid') return false;
    if (paymentFilter === 'pending' && p.status !== 'pending') return false;
    if (paymentFilter === 'expired' && p.status !== 'expired' && p.status !== 'cancelled') return false;
    return true;
  });

  const filteredVouchers = vouchers.filter((v) => {
    if (voucherFilter === 'free' && (v.used_at !== 0 || v.revoked !== 0)) return false;
    if (voucherFilter === 'used' && v.used_at === 0) return false;
    if (voucherFilter === 'revoked' && v.revoked === 0) return false;
    if (voucherSearch.trim()) {
      const q = voucherSearch.toLowerCase();
      return v.code.toLowerCase().includes(q) || (v.note || '').toLowerCase().includes(q);
    }
    return true;
  });

  const handleAdminLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoggingIn(true);
    setLoginError('');
    try {
      const form = new URLSearchParams();
      form.append('username', adminUsername);
      form.append('password', adminPassword);

      const res = await fetch('/admin/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          Accept: 'application/json',
        },
        body: form.toString(),
      });
      const data = await res.json();
      if (res.ok && data.success) {
        setIsAuthenticated(true);
        loadData();
      } else {
        setLoginError(data.message || 'Username atau password salah.');
      }
    } catch {
      setLoginError('Gagal menghubungi server.');
    } finally {
      setLoggingIn(false);
    }
  };

  const handleAdminLogout = async () => {
    await fetch('/keluar', { method: 'POST' });
    setIsAuthenticated(false);
    setAdminPassword('');
  };

  if (!isAuthenticated && !loading) {
    return (
      <div className="max-w-md w-full mx-auto py-8 sm:py-16 px-2">
        <div className="card-bone p-6 sm:p-8">
          <div className="text-center space-y-2 mb-6">
            <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-orange-500 to-amber-600 flex items-center justify-center text-white mx-auto shadow-md shadow-orange-500/25">
              <Shield className="w-6 h-6 stroke-[2.5]" />
            </div>
            <h1 className="text-xl font-extrabold text-[#0f172a] tracking-tight">
              Masuk Administrator
            </h1>
            <p className="text-xs text-[#71717a]">
              Silakan masukkan username dan password admin sistem.
            </p>
          </div>

          {loginError && (
            <div className="mb-5 p-3 rounded-xl bg-red-50 border border-red-200 text-red-700 text-xs flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0" />
              <span>{loginError}</span>
            </div>
          )}

          <form onSubmit={handleAdminLogin} className="space-y-4">
            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                Username
              </label>
              <input
                type="text"
                value={adminUsername}
                onChange={(e) => setAdminUsername(e.target.value)}
                required
                className="input-bone w-full px-3.5 py-2.5 text-sm font-semibold"
                autoFocus
              />
            </div>

            <div>
              <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                Password
              </label>
              <div className="relative">
                <input
                  type={showAdminPassword ? 'text' : 'password'}
                  value={adminPassword}
                  onChange={(e) => setAdminPassword(e.target.value)}
                  required
                  placeholder="••••••••"
                  className="input-bone w-full px-3.5 py-2.5 pr-10 text-sm font-mono"
                />
                <button
                  type="button"
                  onClick={() => setShowAdminPassword(!showAdminPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-[#71717a] hover:text-[#0f172a] p-1"
                >
                  {showAdminPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={loggingIn || !adminUsername.trim() || !adminPassword}
                className="btn-orange w-full py-3 rounded-xl text-xs sm:text-sm font-bold flex items-center justify-center gap-2 shadow-md shadow-orange-500/25"
              >
                {loggingIn ? (
                  <>
                    <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                    <span>Memverifikasi...</span>
                  </>
                ) : (
                  <span>Masuk ke Panel Admin</span>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl w-full mx-auto space-y-5 sm:space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-[#e8e4dc]">
        <div>
          <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-amber-50 border border-amber-200 text-amber-800 text-[11px] font-bold mb-1">
            <Shield className="w-3.5 h-3.5 text-amber-600" />
            <span>Master Administrator</span>
          </div>
          <h1 className="text-xl sm:text-2xl font-extrabold text-[#0f172a] tracking-tight">
            Panel Kelola Sistem rdpinstall
          </h1>
        </div>

        <div className="flex items-center gap-2 self-start sm:self-auto">
          <button
            type="button"
            onClick={loadData}
            disabled={loading}
            className="px-3.5 py-2 rounded-lg border border-[#e8e4dc] bg-white hover:bg-[#f4f1ea] text-xs font-bold text-[#0f172a] flex items-center gap-1.5 transition-all shadow-xs"
          >
            <RefreshCw className={`w-3.5 h-3.5 text-orange-500 ${loading ? 'animate-spin' : ''}`} />
            <span>Segarkan</span>
          </button>

          <button
            type="button"
            onClick={handleAdminLogout}
            className="px-3.5 py-2 rounded-lg border border-red-200 bg-red-50 hover:bg-red-100 text-xs font-bold text-red-700 flex items-center gap-1.5 transition-all shadow-xs"
            title="Keluar dari Panel Admin"
          >
            <LogOut className="w-3.5 h-3.5" />
            <span>Keluar Admin</span>
          </button>
        </div>
      </div>

      {/* Notifications */}
      {notice && (
        <div className="p-3.5 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-800 text-xs sm:text-sm font-semibold flex items-center gap-2 shadow-xs">
          <CheckCircle2 className="w-4 h-4 text-emerald-600 flex-shrink-0" />
          <span>{notice}</span>
        </div>
      )}
      {error && (
        <div className="p-3.5 rounded-xl bg-red-50 border border-red-200 text-red-700 text-xs sm:text-sm flex items-center gap-2 shadow-xs">
          <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* Admin Nav Tabs */}
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
        {[
          { id: 'overview', label: 'Ringkasan', icon: Activity },
          { id: 'jobs', label: 'Instalasi (Jobs)', icon: Server, badge: stats?.active_jobs },
          { id: 'users', label: 'Pengguna', icon: Users, badge: stats?.total_users },
          { id: 'payments', label: 'Pembayaran QRIS', icon: CreditCard },
          { id: 'vouchers', label: 'Kelola Voucer', icon: Ticket, badge: stats?.free_vouchers },
        ].map(({ id, label, icon: Icon, badge }) => (
          <button
            key={id}
            type="button"
            onClick={() => setActiveTab(id as any)}
            className={`px-3 py-2.5 rounded-xl text-xs sm:text-sm font-bold flex items-center justify-center gap-1.5 sm:gap-2 transition-all min-h-[42px] ${
              activeTab === id
                ? 'bg-[#0f172a] text-white shadow-sm'
                : 'card-bone text-[#52525b] hover:text-[#0f172a] hover:bg-[#f4f1ea]'
            }`}
          >
            <Icon className="w-4 h-4 flex-shrink-0 text-orange-500" />
            <span className="truncate">{label}</span>
            {badge !== undefined && badge > 0 && (
              <span
                className={`px-1.5 py-0.2 rounded-full text-[10px] font-mono font-bold flex-shrink-0 ${
                  activeTab === id ? 'bg-orange-500 text-white' : 'bg-[#e8e4dc] text-[#0f172a]'
                }`}
              >
                {badge}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* TAB 1: OVERVIEW */}
      {activeTab === 'overview' && (
        <div className="space-y-5 sm:space-y-6">
          {/* Stat Cards */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 sm:gap-4">
            <div className="card-bone p-4">
              <span className="text-[11px] font-bold text-[#71717a] uppercase block">Total Pengguna</span>
              <div className="text-2xl sm:text-3xl font-extrabold text-[#0f172a] mt-1">
                {stats?.total_users || 0}
              </div>
              <span className="text-[11px] text-[#71717a] mt-1 block">Akun terdaftar via TG</span>
            </div>

            <div className="card-bone p-4">
              <span className="text-[11px] font-bold text-[#71717a] uppercase block">Total Instalasi</span>
              <div className="text-2xl sm:text-3xl font-extrabold text-[#0f172a] mt-1 flex items-baseline gap-2">
                <span>{stats?.total_jobs || 0}</span>
                {stats?.active_jobs ? (
                  <span className="text-xs text-orange-600 font-bold animate-pulse">
                    ({stats.active_jobs} aktif)
                  </span>
                ) : null}
              </div>
              <span className="text-[11px] text-[#71717a] mt-1 block">
                {stats?.done_jobs || 0} sukses · {stats?.failed_jobs || 0} gagal
              </span>
            </div>

            <div className="card-bone p-4">
              <span className="text-[11px] font-bold text-[#71717a] uppercase block">Omset QRIS</span>
              <div className="text-2xl sm:text-3xl font-extrabold text-[#0f172a] mt-1">
                Rp{(stats?.total_revenue || 0).toLocaleString('id-ID')}
              </div>
              <span className="text-[11px] text-[#71717a] mt-1 block">Total topup sukses</span>
            </div>

            <div className="card-bone p-4">
              <span className="text-[11px] font-bold text-[#71717a] uppercase block">Voucer Tersedia</span>
              <div className="text-2xl sm:text-3xl font-extrabold text-orange-600 mt-1">
                {stats?.free_vouchers || 0}
                <span className="text-xs font-normal text-[#71717a] ml-1">/{stats?.total_vouchers || 0}</span>
              </div>
              <span className="text-[11px] text-[#71717a] mt-1 block">Siap digunakan tamu</span>
            </div>
          </div>

          {/* System Info Box */}
          <div className="card-bone p-5 sm:p-6 bg-gradient-to-br from-white to-[#fbfaf7]">
            <h2 className="text-sm font-bold text-[#0f172a] mb-3 flex items-center gap-2">
              <Activity className="w-4 h-4 text-orange-500" />
              <span>Status Operasional Sistem</span>
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
              <div className="p-3 bg-[#f4f1ea] rounded-xl border border-[#e2ded4]">
                <span className="text-[#71717a] block">Tarif Per Instalasi</span>
                <span className="font-bold text-sm text-[#0f172a]">
                  Rp{(stats?.price_per_install || 5000).toLocaleString('id-ID')}
                </span>
              </div>
              <div className="p-3 bg-[#f4f1ea] rounded-xl border border-[#e2ded4]">
                <span className="text-[#71717a] block">Total Saldo Beredar</span>
                <span className="font-bold text-sm text-[#0f172a]">
                  Rp{(stats?.total_balance || 0).toLocaleString('id-ID')}
                </span>
              </div>
              <div className="p-3 bg-[#f4f1ea] rounded-xl border border-[#e2ded4]">
                <span className="text-[#71717a] block">Engine Installer</span>
                <span className="font-mono font-bold text-xs text-[#0f172a]">JayStore rdpinstall v1.0</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: JOBS (INSTALASI) */}
      {activeTab === 'jobs' && (
        <div className="space-y-4">
          <div className="card-bone p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            {/* Filter buttons */}
            <div className="flex flex-wrap items-center gap-1.5">
              {[
                { id: 'all', label: 'Semua' },
                { id: 'running', label: 'Aktif/Berjalan' },
                { id: 'done', label: 'Selesai' },
                { id: 'failed', label: 'Gagal' },
              ].map(({ id, label }) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setJobFilter(id)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${
                    jobFilter === id
                      ? 'bg-orange-500 text-white shadow-xs'
                      : 'bg-[#f4f1ea] text-[#52525b] hover:text-[#0f172a]'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            {/* Search */}
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-[#71717a] absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                placeholder="Cari Host / ID Job..."
                value={jobSearch}
                onChange={(e) => setJobSearch(e.target.value)}
                className="input-bone pl-8 pr-3 py-1.5 text-xs w-full sm:w-56"
              />
            </div>
          </div>

          {/* Job List */}
          <div className="card-bone p-4 sm:p-5">
            <div className="flex items-center justify-between mb-3 text-xs text-[#71717a] font-bold uppercase tracking-wider">
              <span>Daftar Pekerjaan ({filteredJobs.length})</span>
            </div>

            {filteredJobs.length === 0 ? (
              <p className="text-center py-10 text-xs text-[#71717a]">Tidak ada data pekerjaan instalasi.</p>
            ) : (
              <div className="divide-y divide-[#e8e4dc]">
                {filteredJobs.map((j) => (
                  <div key={j.id} className="py-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
                    <div className="space-y-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-sm text-[#0f172a]">#{j.id}</span>
                        <span className="font-bold text-[#0f172a]">{j.os_label}</span>
                        <span
                          className={`px-2 py-0.5 rounded-full text-[10px] font-bold uppercase ${
                            j.status === 'done'
                              ? 'bg-emerald-100 text-emerald-800'
                              : j.status === 'failed' || j.status === 'cancelled'
                              ? 'bg-red-100 text-red-800'
                              : 'bg-orange-100 text-orange-800 animate-pulse'
                          }`}
                        >
                          {j.status}
                        </span>
                        {j.guest ? (
                          <span className="bg-[#f4f1ea] px-1.5 py-0.5 rounded text-[10px] font-mono text-[#71717a]">
                            Tamu (Voucer: {j.voucher_code || '-'})
                          </span>
                        ) : (
                          <span className="bg-blue-50 text-blue-700 px-1.5 py-0.5 rounded text-[10px] font-bold">
                            User #{j.user_id}
                          </span>
                        )}
                      </div>

                      <div className="font-mono text-[#71717a] text-[11px] flex flex-wrap gap-x-3">
                        <span>Target: <b>{j.host}:{j.ssh_port}</b></span>
                        <span>Fase: <b className="text-[#0f172a]">{j.phase || '-'}</b></span>
                        <span>Waktu: {new Date(j.created_at * 1000).toLocaleString('id-ID')}</span>
                      </div>

                      {j.error && (
                        <p className="text-red-600 text-[11px] font-mono break-all mt-0.5">
                          Error: {j.error}
                        </p>
                      )}
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-2 self-end sm:self-auto flex-shrink-0">
                      {(j.status === 'running' || j.status === 'queued') && (
                        <button
                          type="button"
                          onClick={() => handleCancelJob(j.id)}
                          className="px-2.5 py-1.5 rounded-lg border border-red-200 bg-red-50 text-red-700 hover:bg-red-100 text-xs font-bold transition-all"
                        >
                          Batalkan
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={() => handleDeleteJob(j.id)}
                        className="p-1.5 rounded-lg text-[#71717a] hover:text-red-600 hover:bg-red-50 transition-colors"
                        title="Hapus Job"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* TAB 3: USERS */}
      {activeTab === 'users' && (
        <div className="space-y-4">
          <div className="card-bone p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div className="relative flex-1 max-w-sm">
              <Search className="w-3.5 h-3.5 text-[#71717a] absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                placeholder="Cari Username / ID Pengguna..."
                value={userSearch}
                onChange={(e) => setUserSearch(e.target.value)}
                className="input-bone pl-8 pr-3 py-1.5 text-xs w-full"
              />
            </div>
            <span className="text-xs text-[#71717a]">Total {filteredUsers.length} pengguna</span>
          </div>

          <div className="card-bone p-4 sm:p-5">
            {filteredUsers.length === 0 ? (
              <p className="text-center py-10 text-xs text-[#71717a]">Tidak ada pengguna terdaftar.</p>
            ) : (
              <div className="divide-y divide-[#e8e4dc]">
                {filteredUsers.map((u) => (
                  <div key={u.id} className="py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
                    <div className="space-y-0.5">
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-[#0f172a] text-sm">
                          {u.first_name} {u.last_name}
                        </span>
                        {u.username && (
                          <span className="text-orange-600 font-mono font-semibold">@{u.username}</span>
                        )}
                        <span className="font-mono text-[11px] text-[#71717a] bg-[#f4f1ea] px-1.5 py-0.2 rounded">
                          #{u.id}
                        </span>
                        {u.banned !== 0 && (
                          <span className="bg-red-100 text-red-700 px-1.5 py-0.2 rounded font-bold text-[10px]">
                            DIBLOKIR
                          </span>
                        )}
                      </div>
                      <div className="text-[11px] text-[#71717a] flex flex-wrap gap-x-3">
                        <span>TG ID: <b className="font-mono text-[#0f172a]">{u.tg_id || '-'}</b></span>
                        <span>Total Job: <b className="text-[#0f172a]">{u.job_count}</b></span>
                        <span>Daftar: {new Date(u.created_at * 1000).toLocaleDateString('id-ID')}</span>
                      </div>
                    </div>

                    <div className="flex items-center gap-3 self-end sm:self-auto">
                      <div className="text-right">
                        <span className="text-[10px] text-[#71717a] block uppercase font-bold">Saldo</span>
                        <span className="font-bold text-sm text-[#0f172a]">
                          Rp{u.balance.toLocaleString('id-ID')}
                        </span>
                      </div>

                      <button
                        type="button"
                        onClick={() => {
                          setSelectedUserForBalance(u);
                          setBalanceAmount(u.balance);
                          setBalanceMode('add');
                        }}
                        className="px-3 py-1.5 rounded-lg border border-[#e8e4dc] bg-white hover:bg-[#f4f1ea] font-bold text-xs text-[#0f172a] transition-all"
                      >
                        Ubah Saldo
                      </button>

                      <button
                        type="button"
                        onClick={() => handleToggleBan(u.id, u.banned)}
                        className={`p-1.5 rounded-lg transition-colors ${
                          u.banned
                            ? 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100'
                            : 'text-[#71717a] hover:text-red-600 hover:bg-red-50'
                        }`}
                        title={u.banned ? 'Buka Blokir' : 'Blokir User'}
                      >
                        <Ban className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* TAB 4: PAYMENTS */}
      {activeTab === 'payments' && (
        <div className="space-y-4">
          <div className="card-bone p-4 flex items-center justify-between gap-3">
            <div className="flex items-center gap-1.5">
              {[
                { id: 'all', label: 'Semua' },
                { id: 'paid', label: 'Lunas' },
                { id: 'pending', label: 'Menunggu' },
                { id: 'expired', label: 'Kedaluwarsa' },
              ].map(({ id, label }) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setPaymentFilter(id)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${
                    paymentFilter === id
                      ? 'bg-orange-500 text-white shadow-xs'
                      : 'bg-[#f4f1ea] text-[#52525b] hover:text-[#0f172a]'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
            <span className="text-xs text-[#71717a] font-bold">Total {filteredPayments.length} transaksi</span>
          </div>

          <div className="card-bone p-4 sm:p-5">
            {filteredPayments.length === 0 ? (
              <p className="text-center py-10 text-xs text-[#71717a]">Tidak ada data pembayaran.</p>
            ) : (
              <div className="divide-y divide-[#e8e4dc]">
                {filteredPayments.map((p) => (
                  <div key={p.id} className="py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-sm text-[#0f172a]">PG-{p.reference}</span>
                        <span
                          className={`px-2 py-0.5 rounded-full text-[10px] font-bold uppercase ${
                            p.status === 'paid'
                              ? 'bg-emerald-100 text-emerald-800'
                              : p.status === 'pending'
                              ? 'bg-orange-100 text-orange-800 animate-pulse'
                              : 'bg-gray-100 text-gray-700'
                          }`}
                        >
                          {p.status}
                        </span>
                        <span className="text-xs text-[#71717a]">
                          User #{p.user_id} {p.user_username ? `(@${p.user_username})` : ''}
                        </span>
                      </div>
                      <div className="text-[11px] text-[#71717a] flex flex-wrap gap-x-3">
                        <span>Nominal: <b className="text-[#0f172a]">Rp{p.amount.toLocaleString('id-ID')}</b></span>
                        <span>Gateway: <b>{p.gateway}</b></span>
                        <span>Dibuat: {new Date(p.created_at * 1000).toLocaleString('id-ID')}</span>
                        {p.paid_at > 0 && <span>Lunas: {new Date(p.paid_at * 1000).toLocaleString('id-ID')}</span>}
                      </div>
                    </div>

                    <div className="self-end sm:self-auto">
                      {p.status === 'pending' && (
                        <button
                          type="button"
                          onClick={() => handleApprovePayment(p.reference, p.amount)}
                          className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs transition-colors shadow-xs"
                        >
                          Acc Manual (Lunas)
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* TAB 5: VOUCHERS */}
      {activeTab === 'vouchers' && (
        <div className="space-y-5">
          {/* Create voucher card */}
          <div className="card-bone p-4 sm:p-6">
            <h2 className="text-sm font-bold text-[#0f172a] mb-3 flex items-center gap-2">
              <Plus className="w-4 h-4 text-orange-500" />
              <span>Cetak Kode Voucer Baru</span>
            </h2>

            <form onSubmit={handleCreateVouchers} className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Jumlah (1-100)
                </label>
                <input
                  type="number"
                  min="1"
                  max="100"
                  value={voucherCount}
                  onChange={(e) => setVoucherCount(Number(e.target.value))}
                  className="input-bone w-full px-3 py-2 text-sm font-bold font-mono"
                />
              </div>

              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Catatan / Peruntukan
                </label>
                <input
                  type="text"
                  placeholder="Misal: Batch Reseller / Promo TG"
                  value={voucherNote}
                  onChange={(e) => setVoucherNote(e.target.value)}
                  className="input-bone w-full px-3 py-2 text-sm"
                />
              </div>

              <div className="flex items-end">
                <button
                  type="submit"
                  disabled={creatingVouchers}
                  className="btn-orange w-full py-2.5 rounded-lg text-xs font-bold flex items-center justify-center gap-1.5"
                >
                  <Plus className="w-4 h-4" />
                  <span>{creatingVouchers ? 'Mencetak...' : 'Cetak Voucer Sekarang'}</span>
                </button>
              </div>
            </form>
          </div>

          {/* Vouchers list card */}
          <div className="card-bone p-4 sm:p-5">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
              <div className="flex flex-wrap items-center gap-1.5">
                {[
                  { id: 'all', label: 'Semua' },
                  { id: 'free', label: 'Tersedia' },
                  { id: 'used', label: 'Dipakai' },
                  { id: 'revoked', label: 'Dicabut' },
                ].map(({ id, label }) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setVoucherFilter(id)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${
                      voucherFilter === id
                        ? 'bg-orange-500 text-white shadow-xs'
                        : 'bg-[#f4f1ea] text-[#52525b] hover:text-[#0f172a]'
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>

              <div className="relative">
                <Search className="w-3.5 h-3.5 text-[#71717a] absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  type="text"
                  placeholder="Cari Kode Voucer / Note..."
                  value={voucherSearch}
                  onChange={(e) => setVoucherSearch(e.target.value)}
                  className="input-bone pl-8 pr-3 py-1.5 text-xs w-full sm:w-56"
                />
              </div>
            </div>

            {filteredVouchers.length === 0 ? (
              <p className="text-center py-10 text-xs text-[#71717a]">Belum ada voucer yang cocok.</p>
            ) : (
              <div className="divide-y divide-[#e8e4dc] max-h-96 overflow-y-auto pr-1">
                {filteredVouchers.map((v) => (
                  <div key={v.code} className="py-2.5 sm:py-3 flex flex-col sm:flex-row sm:items-center justify-between gap-2 sm:gap-3 text-xs">
                    <div className="flex items-center gap-2 font-mono flex-wrap">
                      <span className="font-bold text-[#0f172a] text-sm">{v.code}</span>
                      <button
                        type="button"
                        onClick={() => copyCode(v.code)}
                        className="p-1 rounded hover:bg-[#f4f1ea] text-[#71717a]"
                        title="Salin"
                      >
                        {copiedCode === v.code ? <Check className="w-3.5 h-3.5 text-emerald-600" /> : <Copy className="w-3.5 h-3.5" />}
                      </button>
                      {v.note && (
                        <span className="text-[11px] font-sans text-[#71717a] bg-[#f4f1ea] px-1.5 py-0.5 rounded break-all">
                          {v.note}
                        </span>
                      )}
                      {v.used_host && (
                        <span className="text-[10px] text-blue-700 bg-blue-50 px-1.5 py-0.5 rounded">
                          Host: {v.used_host}
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 self-end sm:self-auto flex-shrink-0">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase ${
                          v.revoked
                            ? 'bg-red-100 text-red-700'
                            : v.used_at
                            ? 'bg-gray-100 text-gray-600'
                            : 'bg-emerald-100 text-emerald-700'
                        }`}
                      >
                        {v.revoked ? 'Dicabut' : v.used_at ? 'Dipakai' : 'Tersedia'}
                      </span>

                      {!v.revoked && !v.used_at && (
                        <button
                          type="button"
                          onClick={() => handleRevokeVoucher(v.code)}
                          className="p-1 text-red-600 hover:bg-red-50 rounded"
                          title="Cabut Voucer"
                        >
                          <Ban className="w-3.5 h-3.5" />
                        </button>
                      )}

                      {!v.used_at && (
                        <button
                          type="button"
                          onClick={() => handleDeleteVoucher(v.code)}
                          className="p-1 text-[#71717a] hover:text-red-600 hover:bg-red-50 rounded"
                          title="Hapus Voucer"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Modal Edit Saldo */}
      {selectedUserForBalance && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-xs">
          <div className="card-bone w-full max-w-sm p-6 relative">
            <button
              type="button"
              onClick={() => setSelectedUserForBalance(null)}
              className="absolute right-4 top-4 text-[#71717a] hover:text-[#0f172a] p-1 rounded-lg"
            >
              <X className="w-4 h-4" />
            </button>

            <h2 className="text-base font-bold text-[#0f172a] mb-1">
              Ubah Saldo User #{selectedUserForBalance.id}
            </h2>
            <p className="text-xs text-[#71717a] mb-4">
              Nama: <b>{selectedUserForBalance.first_name}</b> · Saldo saat ini: <b>Rp{selectedUserForBalance.balance.toLocaleString('id-ID')}</b>
            </p>

            <form onSubmit={handleSaveBalance} className="space-y-4">
              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => setBalanceMode('add')}
                  className={`py-2 rounded-lg text-xs font-bold border transition-all ${
                    balanceMode === 'add'
                      ? 'border-orange-500 bg-orange-50 text-orange-700'
                      : 'border-[#e8e4dc] bg-white text-[#71717a]'
                  }`}
                >
                  Tambah Saldo (+)
                </button>
                <button
                  type="button"
                  onClick={() => setBalanceMode('set')}
                  className={`py-2 rounded-lg text-xs font-bold border transition-all ${
                    balanceMode === 'set'
                      ? 'border-orange-500 bg-orange-50 text-orange-700'
                      : 'border-[#e8e4dc] bg-white text-[#71717a]'
                  }`}
                >
                  Set Saldo Pasti (=)
                </button>
              </div>

              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Nominal (Rp)
                </label>
                <input
                  type="number"
                  step="1000"
                  value={balanceAmount}
                  onChange={(e) => setBalanceAmount(Number(e.target.value))}
                  className="input-bone w-full px-3 py-2 text-base font-bold font-mono"
                  required
                />
              </div>

              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-[#71717a] mb-1.5">
                  Catatan (Opsional)
                </label>
                <input
                  type="text"
                  placeholder="Misal: Bonus / Kompensasi"
                  value={balanceNote}
                  onChange={(e) => setBalanceNote(e.target.value)}
                  className="input-bone w-full px-3 py-2 text-xs"
                />
              </div>

              <div className="pt-2 flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setSelectedUserForBalance(null)}
                  className="px-4 py-2 rounded-lg border border-[#e8e4dc] text-xs font-bold text-[#52525b] hover:bg-[#f4f1ea]"
                >
                  Batal
                </button>
                <button
                  type="submit"
                  disabled={savingBalance}
                  className="btn-orange px-5 py-2 rounded-lg text-xs font-bold"
                >
                  {savingBalance ? 'Menyimpan...' : 'Simpan Saldo'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
