import React, { useState, useEffect } from 'react';
import {
  Server,
  Wallet,
  LogIn,
  LogOut,
  Ticket,
  LayoutDashboard,
  Menu,
  X,
  User as UserIcon,
  Plus,
} from 'lucide-react';
import { UserProfile } from '../types';

interface NavbarProps {
  currentTab: string;
  setTab: (tab: string) => void;
  user: UserProfile | null;
  onLoginClick: () => void;
  onLogout: () => void;
}

export const Navbar: React.FC<NavbarProps> = ({
  currentTab,
  setTab,
  user,
  onLoginClick,
  onLogout,
}) => {
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(false);

  // Lock body scroll when sidebar is open on mobile
  useEffect(() => {
    if (sidebarOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [sidebarOpen]);

  const handleNav = (tab: string) => {
    setTab(tab);
    setSidebarOpen(false);
  };

  const handleLogin = () => {
    setSidebarOpen(false);
    onLoginClick();
  };

  const handleLogout = () => {
    setSidebarOpen(false);
    onLogout();
  };

  return (
    <>
      <header className="sticky top-0 z-40 bg-[#fbfaf7]/95 backdrop-blur-md border-b border-[#e8e4dc] w-full">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 h-14 sm:h-16 flex items-center justify-between gap-4">
          {/* Brand */}
          <button
            type="button"
            onClick={() => handleNav('guest')}
            className="flex items-center gap-2.5 font-extrabold text-base sm:text-lg tracking-tight text-[#0f172a] hover:opacity-90 transition-opacity flex-shrink-0"
          >
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-orange-500 to-amber-600 flex items-center justify-center text-white shadow-md shadow-orange-500/25 flex-shrink-0">
              <Server className="w-4 h-4 stroke-[2.5]" />
            </div>
            <span>
              rdp<span className="text-orange-600">install</span>
            </span>
          </button>

          {/* Desktop Navigation Links (hidden on mobile) */}
          <nav className="hidden sm:flex items-center gap-2">
            <button
              type="button"
              onClick={() => handleNav('guest')}
              className={`px-3.5 py-1.5 rounded-lg text-sm font-semibold flex items-center gap-1.5 transition-all min-h-[38px] ${
                currentTab === 'guest'
                  ? 'bg-[#0f172a] text-white shadow-sm'
                  : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
              }`}
            >
              <Ticket className="w-4 h-4 text-orange-500" />
              <span>Pasang (Voucer)</span>
            </button>

            {user ? (
              <>
                <button
                  type="button"
                  onClick={() => handleNav('dashboard')}
                  className={`px-3.5 py-1.5 rounded-lg text-sm font-semibold flex items-center gap-1.5 transition-all min-h-[38px] ${
                    currentTab === 'dashboard'
                      ? 'bg-[#0f172a] text-white shadow-sm'
                      : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
                  }`}
                >
                  <LayoutDashboard className="w-4 h-4" />
                  <span>Dashboard</span>
                </button>

                <button
                  type="button"
                  onClick={() => handleNav('topup')}
                  className={`px-3.5 py-1.5 rounded-lg text-sm font-semibold flex items-center gap-1.5 transition-all min-h-[38px] ${
                    currentTab === 'topup'
                      ? 'bg-[#0f172a] text-white shadow-sm'
                      : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
                  }`}
                >
                  <Wallet className="w-4 h-4 text-orange-500" />
                  <span>Saldo: Rp{user.balance.toLocaleString('id-ID')}</span>
                </button>

                <button
                  type="button"
                  onClick={handleLogout}
                  className="p-2 rounded-lg text-[#71717a] hover:bg-[#f4f1ea] hover:text-red-600 transition-colors min-h-[38px] min-w-[38px] flex items-center justify-center"
                  title="Keluar"
                >
                  <LogOut className="w-4 h-4" />
                </button>
              </>
            ) : (
              <button
                type="button"
                onClick={onLoginClick}
                className="ml-2 px-4 py-1.5 rounded-lg text-sm font-semibold border border-[#e8e4dc] bg-white text-[#0f172a] hover:border-[#18181b] hover:shadow-sm flex items-center gap-1.5 transition-all min-h-[38px]"
              >
                <LogIn className="w-4 h-4 text-[#52525b]" />
                <span>Masuk Akun</span>
              </button>
            )}
          </nav>

          {/* Mobile Hamburger Button (visible on mobile only) */}
          <div className="flex sm:hidden items-center gap-2">
            <button
              type="button"
              onClick={() => setSidebarOpen(true)}
              className="p-2 rounded-lg border border-[#e8e4dc] bg-white text-[#0f172a] hover:bg-[#f4f1ea] transition-colors flex items-center justify-center"
              aria-label="Buka Menu"
            >
              <Menu className="w-5 h-5" />
            </button>
          </div>
        </div>
      </header>

      {/* Mobile Sidebar Drawer (Off-Canvas) */}
      {sidebarOpen && (
        <div className="fixed inset-0 z-50 sm:hidden">
          {/* Backdrop Overlay */}
          <div
            className="fixed inset-0 bg-black/50 backdrop-blur-xs transition-opacity animate-in fade-in duration-200"
            onClick={() => setSidebarOpen(false)}
          />

          {/* Sidebar Drawer Container */}
          <div className="fixed inset-y-0 right-0 w-full max-w-xs bg-[#fbfaf7] border-l border-[#e8e4dc] shadow-2xl p-5 flex flex-col justify-between z-10 animate-in slide-in-from-right duration-250">
            {/* Drawer Top */}
            <div className="space-y-5">
              {/* Drawer Header */}
              <div className="flex items-center justify-between pb-3 border-b border-[#e8e4dc]">
                <div className="flex items-center gap-2 font-extrabold text-base tracking-tight text-[#0f172a]">
                  <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-orange-500 to-amber-600 flex items-center justify-center text-white shadow-xs">
                    <Server className="w-3.5 h-3.5 stroke-[2.5]" />
                  </div>
                  <span>
                    rdp<span className="text-orange-600">install</span>
                  </span>
                </div>

                <button
                  type="button"
                  onClick={() => setSidebarOpen(false)}
                  className="p-1.5 rounded-lg text-[#71717a] hover:bg-[#f4f1ea] hover:text-[#0f172a] transition-colors"
                  aria-label="Tutup Menu"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              {/* User Profile Card (if logged in) */}
              {user ? (
                <div className="p-3.5 rounded-xl bg-white border border-[#e8e4dc] space-y-2 shadow-xs">
                  <div className="flex items-center gap-2.5">
                    <div className="w-8 h-8 rounded-full bg-orange-100 text-orange-600 font-bold flex items-center justify-center text-xs">
                      <UserIcon className="w-4 h-4" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="font-bold text-xs text-[#0f172a] truncate">
                        {user.first_name} {user.last_name}
                      </div>
                      <div className="text-[11px] text-[#71717a] font-mono truncate">
                        {user.username ? `@${user.username}` : `user#${user.id}`}
                      </div>
                    </div>
                  </div>

                  <div className="pt-2 border-t border-[#f4f1ea] flex items-center justify-between">
                    <span className="text-[11px] text-[#71717a]">Saldo Akun</span>
                    <span className="text-xs font-bold font-mono text-[#0f172a]">
                      Rp{user.balance.toLocaleString('id-ID')}
                    </span>
                  </div>
                </div>
              ) : (
                <div className="p-3.5 rounded-xl bg-orange-50/60 border border-orange-200/60 text-xs text-orange-900 flex items-center justify-between">
                  <span>Mode Tamu (Tanpa Akun)</span>
                  <button
                    type="button"
                    onClick={handleLogin}
                    className="font-bold text-orange-700 underline text-xs"
                  >
                    Masuk
                  </button>
                </div>
              )}

              {/* Navigation Links in Sidebar */}
              <div className="space-y-1.5">
                <button
                  type="button"
                  onClick={() => handleNav('guest')}
                  className={`w-full p-3 rounded-xl text-left text-xs sm:text-sm font-bold flex items-center gap-3 transition-colors ${
                    currentTab === 'guest'
                      ? 'bg-[#0f172a] text-white shadow-xs'
                      : 'text-[#0f172a] hover:bg-[#f4f1ea]'
                  }`}
                >
                  <Ticket className="w-4 h-4 text-orange-500" />
                  <span>Pasang RDP (Voucer)</span>
                </button>

                {user ? (
                  <>
                    <button
                      type="button"
                      onClick={() => handleNav('dashboard')}
                      className={`w-full p-3 rounded-xl text-left text-xs sm:text-sm font-bold flex items-center gap-3 transition-colors ${
                        currentTab === 'dashboard'
                          ? 'bg-[#0f172a] text-white shadow-xs'
                          : 'text-[#0f172a] hover:bg-[#f4f1ea]'
                      }`}
                    >
                      <LayoutDashboard className="w-4 h-4 text-orange-500" />
                      <span>Dashboard & Riwayat</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => handleNav('topup')}
                      className={`w-full p-3 rounded-xl text-left text-xs sm:text-sm font-bold flex items-center justify-between transition-colors ${
                        currentTab === 'topup'
                          ? 'bg-[#0f172a] text-white shadow-xs'
                          : 'text-[#0f172a] hover:bg-[#f4f1ea]'
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        <Wallet className="w-4 h-4 text-orange-500" />
                        <span>Isi Saldo (QRIS)</span>
                      </div>
                      <Plus className="w-3.5 h-3.5 text-[#71717a]" />
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={handleLogin}
                    className="w-full p-3 rounded-xl text-left text-xs sm:text-sm font-bold flex items-center gap-3 bg-white border border-[#e8e4dc] text-[#0f172a] hover:border-[#0f172a] transition-all"
                  >
                    <LogIn className="w-4 h-4 text-orange-500" />
                    <span>Masuk via Telegram</span>
                  </button>
                )}
              </div>
            </div>

            {/* Drawer Bottom */}
            <div className="pt-4 border-t border-[#e8e4dc] space-y-3">
              {user && (
                <button
                  type="button"
                  onClick={handleLogout}
                  className="w-full py-2.5 px-3 rounded-xl border border-red-200 bg-red-50 hover:bg-red-100 text-red-700 font-bold text-xs flex items-center justify-center gap-2 transition-colors"
                >
                  <LogOut className="w-4 h-4" />
                  <span>Keluar dari Akun</span>
                </button>
              )}

              <p className="text-[11px] text-[#a1a1aa] text-center font-mono">
                rdpinstall v0.1.0 · Universal KVM
              </p>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
