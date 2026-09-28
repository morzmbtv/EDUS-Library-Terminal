import additionalKk from './translations.kk.json';
import { localRequest } from '../api/localTransport.ts';
import { computed, ref } from 'vue';

export type Locale = 'ru' | 'kk';

const fallback: Locale = 'ru';
export const locale = ref<Locale>(fallback);
export const localeSaveFailed = ref(false);
let localeWrite: Promise<void> = Promise.resolve();

/** A deliberately small, local-first translation service. It has no network or OS dependency. */
const kk: Record<string, string> = {
  'Библиотечный терминал': 'Кітапхана терминалы',
  'Выберите действие для работы с книгами': 'Кітаптармен жұмыс істеу үшін әрекетті таңдаңыз',
  'Выдать книги': 'Кітап беру',
  'Оформить выдачу книг ученику по карте': 'Оқушыға карта арқылы кітап беруді рәсімдеу',
  'Принять книги': 'Кітап қабылдау',
  'Оформить возврат книг от ученика': 'Оқушыдан кітап қайтаруды рәсімдеу',
  'Найти книгу': 'Кітапты табу',
  'Проверить наличие в фонде и посмотреть информацию': 'Қордағы бар-жоғын тексеру және мәліметті көру',
  Начать: 'Бастау',
  Поиск: 'Іздеу',
  Назад: 'Артқа',
  Помощь: 'Көмек',
  Настройки: 'Баптаулар',
  'Приложите карту ученика': 'Оқушы картасын жақындатыңыз',
  'Дождитесь, пока появится имя читателя': 'Оқырманның аты шыққанша күтіңіз',
  или: 'немесе',
  'По лицу': 'Бет арқылы',
  'По карте': 'Карта арқылы',
  'Посмотрите в камеру': 'Камераға қараңыз',
  'Камера выключена': 'Камера өшірулі',
  'Открываем камеру…': 'Камера ашылуда…',
  'Камера недоступна. Проверьте подключение и разрешение Windows.':
    'Камера қолжетімсіз. Қосылымды және Windows рұқсатын тексеріңіз.',
  'Провайдер распознавания не подключён': 'Тану провайдері қосылмаған',
  'Распознавание лица пока недоступно': 'Бетті тану әзірше қолжетімсіз',
  'Используйте карту.': 'Картаны пайдаланыңыз.',
  'Приложите карту ученика, чтобы продолжить операцию.':
    'Операцияны жалғастыру үшін оқушы картасын жақындатыңыз.',
  'Язык интерфейса': 'Интерфейс тілі',
  Русский: 'Орысша',
  Қазақша: 'Қазақша',
  'Настройки и диагностика': 'Баптаулар және диагностика',
  'Закрыть настройки': 'Баптауларды жабу',
  Понятно: 'Түсінікті',
  'Закрыть сообщение': 'Хабарламаны жабу',
  'Как работать с терминалом': 'Терминалмен қалай жұмыс істеу керек',
  'Как определить ученика': 'Оқушыны қалай анықтауға болады',
  'Как выдать книги': 'Кітапты қалай беруге болады',
  'Как принять книги': 'Кітапты қалай қабылдауға болады',
  'Как найти книгу': 'Кітапты қалай табуға болады',
  'Тестовый режим': 'Тест режимі',
  'Язык не сохранён. Проверьте связь со службой и выберите язык повторно.':
    'Тіл сақталмады. Қызметпен байланысты тексеріп, тілді қайта таңдаңыз.',
  'Сервисный режим EDUS': 'EDUS қызмет режимі',
  'Тестовые данные · Читатели': 'Тест деректері · Оқырмандар',
  'Добавить читателя': 'Оқырман қосу',
  'Импорт CSV': 'CSV импорттау',
  'Скачать шаблон': 'Үлгіні жүктеп алу',
  'Привязать карту': 'Картаны байланыстыру',
  'Сбросить все тестовые данные': 'Барлық тест деректерін қалпына келтіру',
};

export function t(value: string): string {
  return locale.value === 'kk'
    ? (additionalKk[value as keyof typeof additionalKk] ?? kk[value] ?? value)
    : value;
}
function applyLocale(value: Locale): void {
  locale.value = value;
  document.documentElement.lang = value === 'kk' ? 'kk' : 'ru';
}
export async function hydrateLocale(): Promise<void> {
  try {
    const settings = await localRequest<{ locale: Locale }>('/settings/ui');
    if (settings.locale === 'ru' || settings.locale === 'kk') applyLocale(settings.locale);
  } catch {
    /* Russian remains the safe local fallback until service recovers. */
  }
}
export function setLocale(value: Locale): void {
  applyLocale(value);
  {
    localeWrite = localeWrite.then(async () => {
      try {
        await localRequest('/settings/ui', { method: 'POST', body: JSON.stringify({ locale: value }) }, true);
        if (locale.value === value) localeSaveFailed.value = false;
      } catch {
        if (locale.value === value) localeSaveFailed.value = true;
      }
    });
    return;
  }
}
export const localeTag = computed(() => (locale.value === 'kk' ? 'kk-KZ' : 'ru-RU'));
export function formatDate(value: Date, options: Intl.DateTimeFormatOptions): string {
  if (locale.value === 'kk' && options.month === 'long') {
    const months = [
      'қаңтар',
      'ақпан',
      'наурыз',
      'сәуір',
      'мамыр',
      'маусым',
      'шілде',
      'тамыз',
      'қыркүйек',
      'қазан',
      'қараша',
      'желтоқсан',
    ];
    return [
      options.day ? String(value.getDate()) : '',
      months[value.getMonth()],
      options.year ? `${value.getFullYear()} ж.` : '',
    ]
      .filter(Boolean)
      .join(' ');
  }
  return value.toLocaleDateString(localeTag.value, options);
}
export function formatNumber(value: number): string {
  return new Intl.NumberFormat(localeTag.value).format(value);
}
