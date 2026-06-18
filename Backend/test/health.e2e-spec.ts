import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import request from 'supertest';
import { App } from 'supertest/types';
import { DataSource } from 'typeorm';
import { RedisService } from '../src/redis/redis.service';
import { StellarEventMonitorService } from '../src/stellar-monitor/services/stellar-event-monitor.service';
import { HealthController } from '../src/health/health.controller';

describe('HealthModule (e2e)', () => {
  let app: INestApplication<App>;

  const mockDataSource = {
    query: jest.fn(),
  };

  const mockRedisService = {
    client: {
      ping: jest.fn(),
    },
  };

  const mockStellarMonitorService = {
    getStatus: jest.fn(),
  };

  beforeAll(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      controllers: [HealthController],
      providers: [
        { provide: DataSource, useValue: mockDataSource },
        { provide: RedisService, useValue: mockRedisService },
        {
          provide: StellarEventMonitorService,
          useValue: mockStellarMonitorService,
        },
      ],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('/health/live (GET)', () => {
    it('should return 200 with status ok and timestamp', () => {
      return request(app.getHttpServer())
        .get('/health/live')
        .expect(200)
        .expect((res) => {
          expect(res.body.status).toBe('ok');
          expect(res.body.timestamp).toBeDefined();
          expect(typeof res.body.timestamp).toBe('string');
        });
    });
  });

  describe('/health/ready (GET)', () => {
    it('should return 200 when all dependencies are healthy', async () => {
      mockDataSource.query.mockResolvedValue([{ '1': 1 }]);
      mockRedisService.client.ping.mockResolvedValue('PONG');
      mockStellarMonitorService.getStatus.mockReturnValue({
        isMonitoring: true,
        lastLedgerSequence: 12345,
        horizonUrl: 'https://horizon-testnet.stellar.org',
      });

      const res = await request(app.getHttpServer())
        .get('/health/ready')
        .expect(200);

      expect(res.body.status).toBe('ok');
      expect(res.body.timestamp).toBeDefined();
      expect(res.body.checks.database).toEqual({ status: 'ok' });
      expect(res.body.checks.redis).toEqual({ status: 'ok' });
      expect(res.body.checks.stellarMonitor).toMatchObject({
        status: 'ok',
        isMonitoring: true,
        lastLedgerSequence: 12345,
      });
    });

    it('should return 503 when database is down', async () => {
      mockDataSource.query.mockRejectedValue(new Error('DB connection failed'));
      mockRedisService.client.ping.mockResolvedValue('PONG');
      mockStellarMonitorService.getStatus.mockReturnValue({
        isMonitoring: true,
        lastLedgerSequence: 12345,
        horizonUrl: 'https://horizon-testnet.stellar.org',
      });

      const res = await request(app.getHttpServer())
        .get('/health/ready')
        .expect(503);

      expect(res.body.status).toBe('error');
      expect(res.body.checks.database.status).toBe('error');
      expect(res.body.checks.database.message).toBe('DB connection failed');
      expect(res.body.checks.redis.status).toBe('ok');
    });

    it('should return 503 when redis is down', async () => {
      mockDataSource.query.mockResolvedValue([{ '1': 1 }]);
      mockRedisService.client.ping.mockRejectedValue(
        new Error('Redis connection refused'),
      );
      mockStellarMonitorService.getStatus.mockReturnValue({
        isMonitoring: true,
        lastLedgerSequence: 12345,
        horizonUrl: 'https://horizon-testnet.stellar.org',
      });

      const res = await request(app.getHttpServer())
        .get('/health/ready')
        .expect(503);

      expect(res.body.status).toBe('error');
      expect(res.body.checks.database.status).toBe('ok');
      expect(res.body.checks.redis.status).toBe('error');
      expect(res.body.checks.redis.message).toBe('Redis connection refused');
    });

    it('should return 503 when both database and redis are down', async () => {
      mockDataSource.query.mockRejectedValue(new Error('DB timeout'));
      mockRedisService.client.ping.mockRejectedValue(
        new Error('Redis timeout'),
      );
      mockStellarMonitorService.getStatus.mockReturnValue({
        isMonitoring: true,
        lastLedgerSequence: 12345,
        horizonUrl: 'https://horizon-testnet.stellar.org',
      });

      const res = await request(app.getHttpServer())
        .get('/health/ready')
        .expect(503);

      expect(res.body.status).toBe('error');
      expect(res.body.checks.database.status).toBe('error');
      expect(res.body.checks.redis.status).toBe('error');
    });
  });
});
