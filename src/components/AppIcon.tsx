import {
  Activity,
  BarChart3,
  Bell,
  Bot,
  Boxes,
  Building2,
  CalendarDays,
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
  Home,
  Image,
  Languages,
  LayoutDashboard,
  Megaphone,
  Menu,
  Moon,
  Package,
  QrCode,
  RefreshCw,
  Rocket,
  Save,
  Search,
  Settings,
  ShieldCheck,
  ShoppingCart,
  SlidersHorizontal,
  Sparkles,
  TerminalSquare,
  TrendingUp,
  UserRound,
  Users,
  Zap
} from 'lucide-react';

export type AppIconName =
  | 'activity' | 'analytics' | 'bell' | 'bot' | 'box' | 'building' | 'calendar' | 'catalog' | 'check'
  | 'chevron-left' | 'circle' | 'cloud' | 'cms' | 'code' | 'command' | 'database'
  | 'dashboard' | 'deploy' | 'external' | 'factory' | 'folder' | 'globe' | 'hard-drive'
  | 'home' | 'image' | 'leads' | 'materials' | 'menu' | 'moon' | 'package' | 'qr' | 'refresh'
  | 'save' | 'search' | 'seo' | 'settings' | 'shield' | 'shop' | 'sliders' | 'sparkles'
  | 'suppliers' | 'terminal' | 'trend' | 'users' | 'zap';

const ICONS: Record<AppIconName, typeof LayoutDashboard> = {
  activity: Activity,
  analytics: BarChart3,
  bell: Bell,
  bot: Bot,
  box: Package,
  building: Building2,
  calendar: CalendarDays,
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
  home: Home,
  image: Image,
  leads: Users,
  materials: ShoppingCart,
  menu: Menu,
  moon: Moon,
  package: Package,
  qr: QrCode,
  refresh: RefreshCw,
  save: Save,
  search: Search,
  seo: Megaphone,
  settings: Settings,
  shield: ShieldCheck,
  shop: ShoppingCart,
  sliders: SlidersHorizontal,
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
