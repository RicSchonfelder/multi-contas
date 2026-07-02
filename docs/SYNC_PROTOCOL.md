# SYNC_PROTOCOL — Protocolo de sincronização de perfis

> Fase-alvo: F3 (sync experimental no MVP). Complementa [ARCHITECTURE.md](ARCHITECTURE.md), [DATA_MODEL.md](DATA_MODEL.md) (sync_manifests/snapshots/objects, fencing token) e [SECURITY_MODEL.md](SECURITY_MODEL.md) (envelope encryption — este doc referencia, não redefine).

## Princípios

1. Nunca sincronizar o diretório do Chromium como arquivos comuns — sempre via manifest + snapshot atômico.
2. O **lease é a prevenção primária de conflito**: só o dispositivo detentor do lease ativo pode fazer upload; o fencing token (bigserial, ver DATA_MODEL) acompanha todo manifest e o servidor rejeita tokens menores que o último aceito.
3. Falha de sync **nunca** destrói o estado local (critério de aceitação nº 14 do MVP).
4. Downloads do usuário NÃO sincronizam por padrão. Segredos NUNCA transitam neste protocolo (ficam no cofre do SO + rotas dedicadas cifradas).

## Classes de dados (tratamento distinto)

| Classe | Sincroniza? | Observações |
|--------|------------|-------------|
| Metadados do perfil (nome, tags, status…) | Sim, via API CRUD normal | Não passa pelo protocolo de arquivos |
| Configurações (rede, extensões, favoritos) | Sim, via API | Versionadas no banco |
| Dados persistentes do navegador (cookies, storage, etc.) | Sim, via este protocolo | Cifrados por envelope |
| Downloads | Não (padrão) | Opt-in futuro |
| Segredos | Nunca | Cofre do SO / rotas dedicadas |
| Logs | Não (telemetria separada) | |
| Snapshots | Sim (são o produto do protocolo) | Retenção por plano |

## Fluxo de upload (pós-encerramento do perfil)

1. Perfil encerrado com processo Chromium finalizado e flush do disco confirmado.
2. Cliente monta **inventário**: lista de arquivos elegíveis (allowlist de subdiretórios do user-data-dir; exclui caches recriáveis — `Cache/`, `Code Cache/`, `GPUCache/` — para reduzir volume).
3. Calcula checksum **BLAKE3** por arquivo + hash do manifest.
4. Divide em chunks (4 MiB), comprime com **zstd**, cifra cada chunk com a **DEK do perfil** (AES-256-GCM; DEK protegida pela KEK da organização — ver SECURITY_MODEL).
5. Dedup por `content_hash` **escopado por organização** (decisão do DATA_MODEL): chunks já existentes não re-sobem.
6. Upload **multipart com retomada** (estado de upload persistido localmente; retry com backoff exponencial + jitter).
7. Ao completar: cria `sync_manifest` com `lease_fencing_token`; servidor valida token, marca snapshot como `committed` **atomicamente** (snapshot é imutável após commit).
8. Falha em qualquer passo → snapshot fica `pending` e é retomável ou expirado por GC; nunca há estado meio-aplicado visível.

## Fluxo de download (pré-abertura do perfil)

1. Adquirir lease (ver ARCHITECTURE §lease).
2. Comparar manifest local × último snapshot `committed`.
3. Baixar somente chunks divergentes; verificar checksum de cada chunk e do manifest completo.
4. Aplicar em **diretório de staging**; validar integridade; troca atômica (rename) com backup do estado anterior.
5. Qualquer falha → mantém estado local anterior intacto e registra evento.

## Conflitos

- Prevenção primária: lease exclusivo + fencing token (rejeição server-side de escrita obsoleta).
- Caso residual (ex.: lease expirado indevidamente com upload em trânsito — TC-017): servidor rejeita manifest com token antigo; cliente preserva estado local como **snapshot de conflito** local e notifica o usuário/admin. Nunca merge automático de dados de navegador.

## Versionamento, snapshots e rollback

- Cada upload committed = 1 snapshot imutável, com retenção por plano (EntitlementService).
- Restaurar snapshot = download normal apontando para snapshot antigo + novo lease; gera novo snapshot no próximo encerramento (histórico linear, sem reescrita).
- GC de snapshots expirados respeita política de retenção e legal hold (LGPD).

## Criptografia — declaração honesta

- **O MVP NÃO é criptografia de ponta a ponta.** Os dados são cifrados no cliente com DEK por perfil, mas a KEK da organização é gerenciada pelo KMS da plataforma (necessário para recuperação administrativa e compartilhamento em equipe). A plataforma, portanto, *tem capacidade técnica* de decifrar dados de perfil, sob controles (auditoria, acesso restrito).
- Marketing e docs públicos devem dizer "criptografia em trânsito e em repouso com chaves por perfil", nunca "E2EE".
- Caminho futuro para E2EE opcional (org-managed keys) documentado como possibilidade pós-F7; implica perda de recuperação administrativa — trade-off a ser decidido com clientes.

## Métricas e falhas observáveis

Sync expõe (OpenTelemetry): duração, bytes, chunks dedup, retries, falhas por causa, conflitos, snapshots pendentes antigos. Cenários TC-002 (perda de internet), TC-006 (upload interrompido), TC-012 (restauração), TC-014 (disco cheio) cobertos em TEST_STRATEGY.
