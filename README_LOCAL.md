# Запуск ZKClear API локально

## Быстрый старт

### Вариант 1: Через Docker (рекомендуется)

```bash
# Запустить API
cd core/zkclear-core
./start-local.sh

# Или вручную:
docker-compose -f docker-compose.local.yml up -d

# Просмотр логов
docker-compose -f docker-compose.local.yml logs -f

# Остановка
docker-compose -f docker-compose.local.yml down
```

API будет доступен на `http://localhost:3000`

### Вариант 2: Локально (без Docker)

```bash
cd core/zkclear-core

# Создать директорию для данных
mkdir -p ./data

# Запустить API
RUST_LOG=info \
STORAGE_PATH=./data \
USE_PLACEHOLDER_PROVER=true \
cargo run --package zkclear-api --features rocksdb
```

## Проверка работы

```bash
# Health check
curl http://localhost:3000/health | jq .

# Account state (замените адрес)
curl http://localhost:3000/api/v1/account/0x1234567890123456789012345678901234567890 | jq .
```

## Конфигурация

Переменные окружения (можно задать в `.env` или через docker-compose):

- `RUST_LOG` - уровень логирования (info, debug, trace)
- `STORAGE_PATH` - путь к RocksDB (по умолчанию `./data`)
- `USE_PLACEHOLDER_PROVER` - использовать placeholder proofs (true/false)
- `BLOCK_INTERVAL_SEC` - интервал создания блоков в секундах (по умолчанию 1)
- `MAX_QUEUE_SIZE` - максимальный размер очереди транзакций (по умолчанию 10000)
- `MAX_TXS_PER_BLOCK` - максимальное количество транзакций в блоке (по умолчанию 100)

## База данных

RocksDB встроена в приложение, отдельная БД не требуется. Данные хранятся в директории `./data` (или `/app/data` в Docker).

## Интеграция с фронтендом

Фронтенд настроен на подключение к API на `http://localhost:3000` через rewrites в `next.config.js`.

