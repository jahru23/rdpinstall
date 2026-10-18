import React from 'react';
import { Server, Wallet, Shield, LogIn, LogOut, Ticket, LayoutDashboard } from 'lucide-react';
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
  return (
    <header className="sticky top-0 z-40 bg-[#fbfaf7]/95 backdrop-blur-md border-b border-[#e8e4dc] w-full">
      <div className="max-w-5xl mx-auto px-3 sm:px-6 h-14 sm:h-16 flex items-center justify-between gap-2 sm:gap-4">
        {/* Brand */}
        <button
          onClick={() => setTab('guest')}
          className="flex items-center gap-2 font-extrabold text-base sm:text-lg tracking-tight text-[#0f172a] hover:opacity-90 transition-opacity flex-shrink-0"
        >
          <div className="w-7 h-7 sm:w-8 sm:h-8 rounded-lg bg-gradient-to-br from-orange-500 to-amber-600 flex items-center justify-center text-white shadow-md shadow-orange-500/25 flex-shrink-0">
            <Server className="w-3.5 h-3.5 sm:w-4 sm:h-4 stroke-[2.5]" />
          </div>
          <span>
            rdp<span className="text-orange-600">install</span>
          </span>
        </button>

        {/* Navigation Tabs */}
        <nav className="flex items-center gap-1 sm:gap-2 overflow-x-auto no-scrollbar py-1">
          <button
            onClick={() => setTab('guest')}
            className={`px-2.5 sm:px-3 py-1.5 rounded-lg text-xs sm:text-sm font-semibold flex items-center gap-1.5 transition-all whitespace-nowrap min-h-[36px] ${
              currentTab === 'guest'
                ? 'bg-[#0f172a] text-white shadow-sm'
                : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
            }`}
          >
            <Ticket className="w-3.5 h-3.5 text-orange-500 flex-shrink-0" />
            <span>Pasang<span className="hidden sm:inline"> (Voucer)</span></span>
          </button>

          {user ? (
            <>
              <button
                onClick={() => setTab('dashboard')}
                className={`px-2.5 sm:px-3 py-1.5 rounded-lg text-xs sm:text-sm font-semibold flex items-center gap-1.5 transition-all whitespace-nowrap min-h-[36px] ${
                  currentTab === 'dashboard'
                    ? 'bg-[#0f172a] text-white shadow-sm'
                    : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
                }`}
              >
                <LayoutDashboard className="w-3.5 h-3.5 flex-shrink-0" />
                <span className="hidden sm:inline">Dashboard</span>
              </button>

              <button
                onClick={() => setTab('topup')}
                className={`px-2.5 sm:px-3 py-1.5 rounded-lg text-xs sm:text-sm font-semibold flex items-center gap-1.5 transition-all whitespace-nowrap min-h-[36px] ${
                  currentTab === 'topup'
                    ? 'bg-[#0f172a] text-white shadow-sm'
                    : 'text-[#52525b] hover:bg-[#f4f1ea] hover:text-[#0f172a]'
                }`}
              >
                <Wallet className="w-3.5 h-3.5 text-orange-500 flex-shrink-0" />
                <span>
                  <span className="hidden sm:inline">Saldo: </span>Rp{user.balance.toLocaleString('id-ID')}
                </span>
              </button>

              {user.is_admin && (
                <button
                  onClick={() => setTab('admin')}
                  className={`px-2.5 sm:px-3 py-1.5 rounded-lg text-xs sm:text-sm font-semibold flex items-center gap-1.5 transition-all whitespace-nowrap min-h-[36px] ${
                    currentTab === 'admin'
                      ? 'bg-amber-500 text-white shadow-sm'
                      : 'text-amber-800 bg-amber-50 hover:bg-amber-100 border border-amber-200/80'
                  }`}
                  title="Panel Admin"
                >
                  <Shield className="w-3.5 h-3.5 flex-shrink-0" />
                  <span>Admin</span>
                </button>
              )}

              <button
                onClick={onLogout}
                className="p-2 rounded-lg text-[#71717a] hover:bg-[#f4f1ea] hover:text-red-600 transition-colors flex-shrink-0 min-h-[36px] min-w-[36px] flex items-center justify-center"
                title="Keluar"
              >
                <LogOut className="w-4 h-4" />
              </button>
            </>
          ) : (
            <button
              onClick={onLoginClick}
              className="px-2.5 sm:px-3.5 py-1.5 rounded-lg text-xs sm:text-sm font-semibold border border-[#e8e4dc] bg-white text-[#0f172a] hover:border-[#18181b] hover:shadow-sm flex items-center gap-1.5 transition-all whitespace-nowrap min-h-[36px]"
            >
              <LogIn className="w-3.5 h-3.5 text-[#52525b] flex-shrink-0" />
              <span>Masuk<span className="hidden sm:inline"> Akun</span></span>
            </button>
          )}
        </nav>
      </div>
    </header>
  );
};
