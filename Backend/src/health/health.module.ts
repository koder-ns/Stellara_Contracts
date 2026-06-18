import { Module } from '@nestjs/common';
import { StellarMonitorModule } from '../stellar-monitor/stellar-monitor.module';
import { HealthController } from './health.controller';

@Module({
  imports: [StellarMonitorModule],
  controllers: [HealthController],
})
export class HealthModule {}
