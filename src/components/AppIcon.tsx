import {
  Activity,
  BarChart3,
  Bell,
  Bot,
  Boxes,
  Building2,
  CheckCircle2,
  ChevronLeft,
  CircleDot,
  Cloud,
  Code2,
  Command,
  Database,
  ExternalLink,
  Factory,
  FileText,
  FolderKanban,
  Globe2,
  HardDrive,
  LayoutDashboard,
  Megaphone,
  Moon,
  Package,
  RefreshCw,
  Rocket,
  Search,
  Settings,
  ShieldCheck,
  ShoppingCart,
  Sparkles,
  TerminalSquare,
  TrendingUp,
  UserRound,
  Users,
  Zap
} from 'lucide-react';

export type AppIconName =
  | 'activity' | 'analytics' | 'bell' | 'bot' | 'box' | 'building' | 'catalog' | 'check'
  | 'chevron-left' | 'circle' | 'cloud' | 'cms' | 'code' | 'command' | 'database'
  | 'dashboard' | 'deploy' | 'external' | 'factory' | 'folder' | 'globe' | 'hard-drive'
  | 'leads' | 'materials' | 'moon' | 'package' | 'refresh' | 'search' | 'seo' | 'settings'
  | 'shield' | 'shop' | 'sparkles' | 'suppliers' | 'terminal' | 'trend' | 'users' | 'zap';

const ICONS: Record<AppIconName, typeof LayoutDashboard> = {
  activity: Activity,
  analytics: BarChart3,
  bell: Bell,
  bot: Bot,
  box: Package,
  building: Building2,
  catalog: Boxes,
  check: CheckCircle2,
  'chevron-left': ChevronLeft,
  circle: CircleDot,
  cloud: Cloud,
  cms: FileText,
  code: Code2,
  command: Command,
  database: Database,
  dashboard: LayoutDashboard,
  deploy: Rocket,
  external: ExternalLink,
  factory: Factory,
  folder: FolderKanban,
  globe: Globe2,
  'hard-drive': HardDrive,
  leads: Users,
  materials: ShoppingCart,
  moon: Moon,
  package: Package,
  refresh: RefreshCw,
  search: Search,
  seo: Megaphone,
  settings: Settings,
  shield: ShieldCheck,
  shop: ShoppingCart,
  sparkles: Sparkles,
  suppliers: Building2,
  terminal: TerminalSquare,
  trend: TrendingUp,
  users: UserRound,
  zap: Zap
};

interface AppIconProps { name: AppIconName; size?: number; }

export function AppIcon({ name, size = 17 }: AppIconProps) {
  const Icon = ICONS[name];
  return <Icon className="app-icon" size={size} strokeWidth={1.8} aria-hidden="true" />;
}
