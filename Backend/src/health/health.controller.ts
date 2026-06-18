import {
  Controller,
  Get,
  Logger,
  ServiceUnavailableException,
} from '@nestjs/common';
import { DataSource } from 'typeorm';
import { RedisService } from '../redis/redis.service';
import { StellarEventMonitorService } from '../stellar-monitor/services/stellar-event-monitor.service';

@Controller('health')
export class HealthController {
  private readonly logger = new Logger(HealthController.name);

  constructor(
    private readonly dataSource: DataSource,
    private readonly redisService: RedisService,
    private readonly stellarMonitorService: StellarEventMonitorService,
  ) {}

  @Get('live')
  getLiveness() {
    return { status: 'ok', timestamp: new Date().toISOString() };
  }

  @Get('ready')
  async getReadiness() {
    const checks: Record<string, { status: string; message?: string; [key: string]: any }> = {};
    let allHealthy = true;

    try {
      await this.dataSource.query('SELECT 1');
      checks.database = { status: 'ok' };
    } catch (err: any) {
      this.logger.warn(`Database health check failed: ${err.message}`);
      checks.database = { status: 'error', message: err.message };
      allHealthy = false;
    }

    try {
      await this.redisService.client.ping();
      checks.redis = { status: 'ok' };
    } catch (err: any) {
      this.logger.warn(`Redis health check failed: ${err.message}`);
      checks.redis = { status: 'error', message: err.message };
      allHealthy = false;
    }

    try {
      const monitorStatus = this.stellarMonitorService.getStatus();
      checks.stellarMonitor = {
        status: monitorStatus.isMonitoring ? 'ok' : 'degraded',
        isMonitoring: monitorStatus.isMonitoring,
        lastLedgerSequence: monitorStatus.lastLedgerSequence,
        horizonUrl: monitorStatus.horizonUrl,
      };
    } catch (err: any) {
      this.logger.warn(`Stellar monitor health check failed: ${err.message}`);
      checks.stellarMonitor = { status: 'error', message: err.message };
    }

    const response = {
      status: allHealthy ? 'ok' : 'error',
      timestamp: new Date().toISOString(),
      checks,
    };

    if (!allHealthy) {
      throw new ServiceUnavailableException(response);
    }

    return response;
  }
}
